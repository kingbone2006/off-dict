//! Xây `dict.sqlite` từ JSONL và tạo FST.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use vietdict_core::normalize::{normalize, strip_diacritics};
use vietdict_core::schema::{build_fsts, CREATE_INDEXES, CREATE_TABLES, SCHEMA_VERSION};

use crate::record::Record;

/// Xây database tái lập được từ các record đã merge.
pub fn build(records: &[Record], out_dir: &Path, data_version: &str) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let db = out_dir.join("dict.sqlite");
    if db.exists() {
        std::fs::remove_file(&db)?;
    }
    let conn = Connection::open(&db).with_context(|| format!("tạo {}", db.display()))?;
    conn.execute_batch(CREATE_TABLES)?;
    let tx = conn.unchecked_transaction()?;
    let mut next_id: i64 = 1;
    let mut inflections = BTreeMap::<(String, String), String>::new();
    for rec in records {
        let lang = rec.lang.as_str();
        if !matches!(lang, "en" | "vi") || rec.headword.trim().is_empty() {
            continue;
        }
        let head = rec.headword.trim();
        tx.execute(
            "INSERT INTO entry(id,headword,headword_norm,lang,ipa_us,ipa_uk,frequency,level) VALUES(?1,?2,?3,?4,?5,?6,?7,NULL)",
            params![next_id, head, strip_diacritics(head), lang, rec.ipa_us, rec.ipa_uk, rec.frequency.unwrap_or(0)],
        )?;
        for (ord, sense) in rec.senses.iter().enumerate() {
            tx.execute(
                "INSERT INTO sense(id,entry_id,pos,gloss,domain,ord) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    next_id * 1000 + ord as i64,
                    next_id,
                    sense.pos,
                    sense.gloss,
                    sense.domain,
                    ord as i64
                ],
            )?;
            let sid = next_id * 1000 + ord as i64;
            for ex in &sense.examples {
                tx.execute(
                    "INSERT INTO example(sense_id,src_text,dst_text) VALUES(?1,?2,?3)",
                    params![sid, ex.src, ex.dst],
                )?;
            }
        }
        for rel in &rec.relations {
            tx.execute(
                "INSERT INTO relation(entry_id,target,type) VALUES(?1,?2,?3)",
                params![next_id, rel.target, rel.kind],
            )?;
        }
        for inf in &rec.inflections {
            inflections.insert((inf.form.clone(), inf.lemma.clone()), inf.kind.clone());
        }
        next_id += 1;
    }
    for ((form, lemma), kind) in inflections {
        tx.execute(
            "INSERT OR IGNORE INTO inflection(form,lemma,kind) VALUES(?1,?2,?3)",
            params![form, lemma, kind],
        )?;
    }
    tx.execute_batch(CREATE_INDEXES)?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES('schema_version',?1)",
        [SCHEMA_VERSION],
    )?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES('data_version',?1)",
        [data_version],
    )?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES('sources','OVDP/Hồ Ngọc Đức; CMUdict')",
        [],
    )?;
    tx.execute_batch("ANALYZE; VACUUM;")?;
    drop(tx);
    build_fsts(&conn, out_dir).map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

/// Đọc nhiều JSONL và build database.
pub fn build_from_jsonl(
    paths: &[std::path::PathBuf],
    out_dir: &Path,
    version: &str,
) -> Result<usize> {
    let mut records = Vec::new();
    for path in paths {
        crate::record::read_jsonl(path, |r| {
            records.push(r);
            Ok(())
        })?;
    }
    build(&records, out_dir, version)?;
    Ok(records.len())
}

#[allow(dead_code)]
fn _normalize_marker(s: &str) -> String {
    normalize(s)
}
