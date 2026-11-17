//! # vietdict-core
//!
//! Lõi tra cứu từ điển Anh ↔ Việt offline: chuẩn hoá, telex, lemmatizer,
//! tìm kiếm (exact / không dấu / lemma / prefix / fuzzy / FTS), xếp hạng,
//! kho dữ liệu người dùng. Không phụ thuộc UI, không gọi mạng.

pub mod dict_repo;
pub mod lemma;
pub mod normalize;
pub mod rank;
pub mod schema;
pub mod search;
pub mod telex;
pub mod tts;
pub mod user_repo;

pub use dict_repo::Dictionary;
pub use user_repo::{HistoryItem, UserStore};

use std::fmt;

/// Ngôn ngữ của mục từ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {
    /// Tiếng Anh.
    En,
    /// Tiếng Việt.
    Vi,
}

impl Lang {
    /// Mã ngôn ngữ dùng trong CSDL (`"en"` / `"vi"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Vi => "vi",
        }
    }

    /// Đọc mã ngôn ngữ từ chuỗi.
    pub fn parse(s: &str) -> Option<Lang> {
        match s {
            "en" => Some(Lang::En),
            "vi" => Some(Lang::Vi),
            _ => None,
        }
    }

    /// Ngôn ngữ còn lại.
    pub fn other(self) -> Lang {
        match self {
            Lang::En => Lang::Vi,
            Lang::Vi => Lang::En,
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Kiểu khớp tốt nhất của một gợi ý.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// Khớp chính xác (có dấu).
    Exact,
    /// Khớp sau khi bỏ dấu.
    NoDiacritics,
    /// Khớp qua chuyển Telex.
    Telex,
    /// Khớp qua lemma (went → go).
    Lemma,
    /// Khớp tiền tố.
    Prefix,
    /// Khớp gần đúng (gõ sai).
    Fuzzy,
    /// Khớp trong phần nghĩa (tra ngược).
    Reverse,
}

/// Một dòng gợi ý.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// `entry.id`.
    pub entry_id: i64,
    /// Từ hiển thị.
    pub headword: String,
    /// Ngôn ngữ.
    pub lang: Lang,
    /// Nghĩa ngắn để xem trước.
    pub preview: String,
    /// Điểm xếp hạng.
    pub score: f64,
    /// Kiểu khớp.
    pub kind: MatchKind,
}

/// Ví dụ minh hoạ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    /// Câu nguồn.
    pub src: String,
    /// Câu dịch (nếu có).
    pub dst: Option<String>,
}

/// Một nghĩa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sense {
    /// Từ loại (noun, verb...).
    pub pos: Option<String>,
    /// Nội dung nghĩa.
    pub gloss: String,
    /// Lĩnh vực.
    pub domain: Option<String>,
    /// Ví dụ.
    pub examples: Vec<Example>,
}

/// Quan hệ với từ khác.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    /// Từ đích.
    pub target: String,
    /// Loại: syn, ant, derived, phrasal, idiom.
    pub kind: String,
}

/// Mục từ đầy đủ.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// `entry.id`.
    pub id: i64,
    /// Từ hiển thị.
    pub headword: String,
    /// Ngôn ngữ.
    pub lang: Lang,
    /// Phiên âm Mỹ.
    pub ipa_us: Option<String>,
    /// Phiên âm Anh.
    pub ipa_uk: Option<String>,
    /// Độ phổ biến (ước lượng).
    pub frequency: i64,
    /// Danh sách nghĩa theo thứ tự.
    pub senses: Vec<Sense>,
    /// Quan hệ.
    pub relations: Vec<Relation>,
    /// Lemma nếu từ này là dạng biến thể (went → go).
    pub lemmas: Vec<String>,
}

/// Lỗi truy cập từ điển.
#[derive(Debug, thiserror::Error)]
pub enum DictError {
    /// Lỗi SQLite.
    #[error("lỗi CSDL: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Lỗi đọc file.
    #[error("lỗi đọc file {path}: {source}")]
    Io {
        /// Đường dẫn.
        path: String,
        /// Lỗi gốc.
        source: std::io::Error,
    },
    /// Lỗi FST.
    #[error("lỗi chỉ mục FST: {0}")]
    Fst(#[from] fst::Error),
    /// Thiếu file dữ liệu.
    #[error("không tìm thấy dữ liệu: {0}")]
    Missing(String),
    /// Phiên bản schema không hỗ trợ.
    #[error("schema_version {found} không được hỗ trợ (cần {expected})")]
    Schema {
        /// Tìm thấy.
        found: String,
        /// Mong đợi.
        expected: &'static str,
    },
    /// Khoá nội bộ bị hỏng do thread khác panic.
    #[error("khoá kết nối bị hỏng")]
    Poisoned,
}

/// Lỗi kho người dùng.
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    /// Lỗi SQLite.
    #[error("lỗi CSDL người dùng: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Lỗi tạo thư mục.
    #[error("lỗi tạo thư mục: {0}")]
    Io(#[from] std::io::Error),
    /// Khoá nội bộ bị hỏng.
    #[error("khoá kết nối bị hỏng")]
    Poisoned,
}
