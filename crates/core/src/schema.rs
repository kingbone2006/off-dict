//! Schema `dict.sqlite` và trình build FST — dùng chung cho builder, app và test.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use rusqlite::Connection;

use crate::{DictError, Lang};

/// Phiên bản schema hiện tại.
pub const SCHEMA_VERSION: &str = "1";

/// Tên file CSDL.
pub const DICT_FILE: &str = "dict.sqlite";

/// Tên file FST theo ngôn ngữ.
pub fn fst_file(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en.fst",
        Lang::Vi => "vi.fst",
    }
}

/// Bảng (tạo trước khi chèn dữ liệu).
pub const CREATE_TABLES: &str = "
CREATE TABLE entry (
  id            INTEGER PRIMARY KEY,
  headword      TEXT NOT NULL,
  headword_norm TEXT NOT NULL,
  lang          TEXT NOT NULL CHECK (lang IN ('en','vi')),
  ipa_us        TEXT,
  ipa_uk        TEXT,
  frequency     INTEGER DEFAULT 0,
  level         TEXT
);
CREATE TABLE sense (
  id        INTEGER PRIMARY KEY,
  entry_id  INTEGER NOT NULL REFERENCES entry(id),
  pos       TEXT,
  gloss     TEXT NOT NULL,
  domain    TEXT,
  ord       INTEGER NOT NULL
);
CREATE TABLE example (
  id        INTEGER PRIMARY KEY,
  sense_id  INTEGER NOT NULL REFERENCES sense(id),
  src_text  TEXT NOT NULL,
  dst_text  TEXT
);
CREATE TABLE inflection (
  form   TEXT NOT NULL,
  lemma  TEXT NOT NULL,
  kind   TEXT,
  PRIMARY KEY (form, lemma)
) WITHOUT ROWID;
CREATE TABLE relation (
  entry_id INTEGER NOT NULL,
  target   TEXT NOT NULL,
  type     TEXT NOT NULL CHECK (type IN ('syn','ant','derived','phrasal','idiom'))
);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
";

/// Index + FTS (tạo sau khi chèn xong).
pub const CREATE_INDEXES: &str = "
CREATE INDEX idx_entry_norm ON entry(lang, headword_norm);
CREATE INDEX idx_sense_entry ON sense(entry_id, ord);
CREATE INDEX idx_example_sense ON example(sense_id);
CREATE INDEX idx_relation_entry ON relation(entry_id);
CREATE VIRTUAL TABLE sense_fts USING fts5(
  gloss, content='sense', content_rowid='id',
  tokenize = 'unicode61 remove_diacritics 2'
);
INSERT INTO sense_fts(sense_fts) VALUES('rebuild');
INSERT INTO sense_fts(sense_fts) VALUES('optimize');
";

/// Giá trị FST: 32 bit thấp = `entry.id`, 32 bit cao = `frequency`.
pub fn pack_value(entry_id: i64, frequency: i64) -> u64 {
    let id = (entry_id.clamp(0, u32::MAX as i64)) as u64;
    let freq = (frequency.clamp(0, u32::MAX as i64)) as u64;
    (freq << 32) | id
}

/// Giải mã giá trị FST → (`entry.id`, `frequency`).
pub fn unpack_value(v: u64) -> (i64, i64) {
    ((v & 0xFFFF_FFFF) as i64, (v >> 32) as i64)
}

/// Build `en.fst` và `vi.fst` từ CSDL đã có dữ liệu.
///
/// Key = `headword_norm`, value = mục có `frequency` cao nhất (xem [`pack_value`]).
pub fn build_fsts(conn: &Connection, out_dir: &Path) -> Result<(), DictError> {
    for lang in [Lang::En, Lang::Vi] {
        let path = out_dir.join(fst_file(lang));
        let file = File::create(&path).map_err(|e| DictError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let mut builder = fst::MapBuilder::new(BufWriter::new(file))?;
        // Sắp xếp theo byte (BLOB) — yêu cầu của fst; ưu tiên frequency cao, id nhỏ.
        let mut stmt = conn.prepare(
            "SELECT headword_norm, id, frequency FROM entry WHERE lang = ?1
             ORDER BY CAST(headword_norm AS BLOB), frequency DESC, id",
        )?;
        let mut rows = stmt.query([lang.as_str()])?;
        let mut last: Option<String> = None;
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            if key.is_empty() || last.as_deref() == Some(key.as_str()) {
                continue;
            }
            builder.insert(key.as_bytes(), pack_value(row.get(1)?, row.get(2)?))?;
            last = Some(key);
        }
        builder.finish()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_roundtrip() {
        assert_eq!(unpack_value(pack_value(123, 45)), (123, 45));
        assert_eq!(unpack_value(pack_value(-1, -1)), (0, 0));
    }
}
