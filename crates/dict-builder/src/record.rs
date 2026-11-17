//! Định dạng JSONL trung gian (xem `docs/DATA_FORMAT.md`).

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Ví dụ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleRec {
    pub src: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dst: Option<String>,
}

/// Nghĩa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenseRec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
    pub gloss: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ExampleRec>,
}

/// Quan hệ.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationRec {
    pub target: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Biến thể: `form` là dạng biến đổi của `lemma`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InflectionRec {
    pub form: String,
    pub lemma: String,
    pub kind: String,
}

/// Một mục từ trong JSONL.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub headword: String,
    pub lang: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipa_us: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipa_uk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senses: Vec<SenseRec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<RelationRec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inflections: Vec<InflectionRec>,
    pub source: String,
}

/// Ghi danh sách bản ghi ra JSONL.
pub fn write_jsonl<'a>(path: &Path, recs: impl IntoIterator<Item = &'a Record>) -> Result<usize> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let f = File::create(path).with_context(|| format!("tạo {}", path.display()))?;
    let mut w = BufWriter::new(f);
    let mut n = 0;
    for r in recs {
        serde_json::to_writer(&mut w, r)?;
        w.write_all(b"\n")?;
        n += 1;
    }
    w.flush()?;
    Ok(n)
}

/// Đọc JSONL theo kiểu streaming, gọi `f` cho từng bản ghi.
pub fn read_jsonl(path: &Path, mut f: impl FnMut(Record) -> Result<()>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("mở {}", path.display()))?;
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let rec: Record = serde_json::from_str(&line)
            .with_context(|| format!("{}:{}: JSON không hợp lệ", path.display(), i + 1))?;
        f(rec)?;
    }
    Ok(())
}
