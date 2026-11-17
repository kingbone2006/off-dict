//! Truy cập `dict.sqlite` (chỉ đọc) + `en.fst` / `vi.fst` (mmap).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, TryLockError};

use fst::Map;
use memmap2::Mmap;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::normalize::{normalize, strip_diacritics};
use crate::schema::{fst_file, DICT_FILE, SCHEMA_VERSION};
use crate::{DictError, Entry, Example, Lang, Relation, Sense};

/// Số kết nối SQLite trong pool nhỏ (thread UI + thread nền).
const POOL_SIZE: usize = 3;

/// Một mục rút gọn (dùng khi xếp hạng).
#[derive(Debug, Clone)]
pub(crate) struct EntryHead {
    pub id: i64,
    pub headword: String,
    pub frequency: i64,
}

/// Từ điển: `dict.sqlite` + `en.fst` + `vi.fst`. `Send + Sync`.
pub struct Dictionary {
    pool: Vec<Mutex<Connection>>,
    fst_en: Map<Mmap>,
    fst_vi: Map<Mmap>,
    data_dir: PathBuf,
}

fn open_fst(path: &Path) -> Result<Map<Mmap>, DictError> {
    let io_err = |e| DictError::Io {
        path: path.display().to_string(),
        source: e,
    };
    let file = File::open(path).map_err(io_err)?;
    // SAFETY: file chỉ đọc, phân phối cùng app và không bị sửa khi app đang chạy.
    // Nếu bị thay giữa chừng, `fst` kiểm tra cấu trúc nên tệ nhất là trả lỗi/kết quả sai.
    let mmap = unsafe { Mmap::map(&file) }.map_err(io_err)?;
    Ok(Map::new(mmap)?)
}

fn open_conn(db: &Path) -> Result<Connection, DictError> {
    let path = db.display().to_string().replace('\\', "/");
    let path = path
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23");
    let prefix = if path.starts_with('/') { "" } else { "/" };
    let uri = format!("file:{prefix}{path}?mode=ro&immutable=1");
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(uri, flags)?;
    conn.execute_batch(
        "PRAGMA mmap_size = 268435456; PRAGMA cache_size = -8000; PRAGMA query_only = 1;",
    )?;
    conn.set_prepared_statement_cache_capacity(32);
    Ok(conn)
}

/// Cắt chuỗi còn tối đa `n` ký tự, thêm `…` nếu bị cắt.
pub(crate) fn truncate_chars(s: &str, n: usize) -> String {
    match s.char_indices().nth(n) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

impl Dictionary {
    /// Mở từ điển trong thư mục `data_dir` (chứa `dict.sqlite`, `en.fst`, `vi.fst`).
    pub fn open(data_dir: &Path) -> Result<Self, DictError> {
        let db = data_dir.join(DICT_FILE);
        let en = data_dir.join(fst_file(Lang::En));
        let vi = data_dir.join(fst_file(Lang::Vi));
        for p in [&db, &en, &vi] {
            if !p.is_file() {
                return Err(DictError::Missing(p.display().to_string()));
            }
        }
        let first = open_conn(&db)?;
        let version: Option<String> = first
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let version = version.unwrap_or_default();
        if version != SCHEMA_VERSION {
            return Err(DictError::Schema {
                found: version,
                expected: SCHEMA_VERSION,
            });
        }
        let mut pool = vec![Mutex::new(first)];
        for _ in 1..POOL_SIZE {
            pool.push(Mutex::new(open_conn(&db)?));
        }
        Ok(Self {
            pool,
            fst_en: open_fst(&en)?,
            fst_vi: open_fst(&vi)?,
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Thư mục dữ liệu đang dùng.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub(crate) fn fst(&self, lang: Lang) -> &Map<Mmap> {
        match lang {
            Lang::En => &self.fst_en,
            Lang::Vi => &self.fst_vi,
        }
    }

    /// Lấy một kết nối rảnh trong pool (chờ kết nối đầu nếu tất cả đang bận).
    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, DictError> {
        for m in &self.pool {
            match m.try_lock() {
                Ok(g) => return Ok(g),
                Err(TryLockError::WouldBlock) => continue,
                Err(TryLockError::Poisoned(_)) => return Err(DictError::Poisoned),
            }
        }
        self.pool[0].lock().map_err(|_| DictError::Poisoned)
    }

    /// Giá trị trong bảng `meta` (vd. `data_version`, `sources`).
    pub fn meta(&self, key: &str) -> Result<Option<String>, DictError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached("SELECT value FROM meta WHERE key = ?1")?;
        Ok(st.query_row([key], |r| r.get(0)).optional()?)
    }

    /// Số mục từ theo ngôn ngữ.
    pub fn entry_count(&self, lang: Lang) -> Result<i64, DictError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached("SELECT COUNT(*) FROM entry WHERE lang = ?1")?;
        Ok(st.query_row([lang.as_str()], |r| r.get(0))?)
    }

    /// Tất cả mục có `headword_norm` = `norm`.
    pub(crate) fn heads_by_norm(
        conn: &Connection,
        lang: Lang,
        norm: &str,
    ) -> Result<Vec<EntryHead>, DictError> {
        let mut st = conn.prepare_cached(
            "SELECT id, headword, frequency FROM entry
             WHERE lang = ?1 AND headword_norm = ?2 ORDER BY frequency DESC, id LIMIT 24",
        )?;
        let rows = st.query_map(rusqlite::params![lang.as_str(), norm], |r| {
            Ok(EntryHead {
                id: r.get(0)?,
                headword: r.get(1)?,
                frequency: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Lemma của một dạng biến thể theo bảng `inflection`: `(lemma, kind)`.
    pub(crate) fn inflection_lemmas(
        conn: &Connection,
        form: &str,
    ) -> Result<Vec<(String, Option<String>)>, DictError> {
        let mut st = conn
            .prepare_cached("SELECT lemma, kind FROM inflection WHERE form = ?1 ORDER BY lemma")?;
        let rows = st.query_map([form], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Lấy (headword, frequency) theo id.
    pub(crate) fn head_by_id(conn: &Connection, id: i64) -> Result<Option<EntryHead>, DictError> {
        let mut st = conn.prepare_cached("SELECT headword, frequency FROM entry WHERE id = ?1")?;
        Ok(st
            .query_row([id], |r| {
                Ok(EntryHead {
                    id,
                    headword: r.get(0)?,
                    frequency: r.get(1)?,
                })
            })
            .optional()?)
    }

    /// Nghĩa đầu tiên (rút gọn) để xem trước.
    pub(crate) fn preview(conn: &Connection, entry_id: i64) -> Result<String, DictError> {
        let mut st = conn
            .prepare_cached("SELECT gloss FROM sense WHERE entry_id = ?1 ORDER BY ord LIMIT 3")?;
        let glosses: Vec<String> = st
            .query_map([entry_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(truncate_chars(&glosses.join("; "), 90))
    }
}

impl Dictionary {
    /// Tra chi tiết một từ. Ưu tiên mục khớp đúng dấu, sau đó mục phổ biến nhất.
    pub fn lookup(&self, headword: &str, lang: Lang) -> Result<Option<Entry>, DictError> {
        let q = normalize(headword);
        if q.is_empty() {
            return Ok(None);
        }
        let id = {
            let conn = self.conn()?;
            let heads = Self::heads_by_norm(&conn, lang, &strip_diacritics(&q))?;
            heads
                .iter()
                .find(|h| normalize(&h.headword) == q)
                .or_else(|| heads.first())
                .map(|h| h.id)
        };
        match id {
            Some(id) => self.lookup_id(id),
            None => Ok(None),
        }
    }

    /// Tra chi tiết theo `entry.id`.
    pub fn lookup_id(&self, id: i64) -> Result<Option<Entry>, DictError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "SELECT headword, lang, ipa_us, ipa_uk, frequency FROM entry WHERE id = ?1",
        )?;
        type Head = (String, String, Option<String>, Option<String>, i64);
        let head: Option<Head> = st
            .query_row([id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .optional()?;
        let Some((headword, lang, ipa_us, ipa_uk, frequency)) = head else {
            return Ok(None);
        };
        let lang = Lang::parse(&lang).unwrap_or(Lang::En);

        let mut st = conn.prepare_cached(
            "SELECT id, pos, gloss, domain FROM sense WHERE entry_id = ?1 ORDER BY ord",
        )?;
        type RawSense = (i64, Option<String>, String, Option<String>);
        let raw: Vec<RawSense> = st
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<_, _>>()?;
        let mut ex_st = conn.prepare_cached(
            "SELECT src_text, dst_text FROM example WHERE sense_id = ?1 ORDER BY id",
        )?;
        let mut senses = Vec::with_capacity(raw.len());
        for (sid, pos, gloss, domain) in raw {
            let examples = ex_st
                .query_map([sid], |r| {
                    Ok(Example {
                        src: r.get(0)?,
                        dst: r.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            senses.push(Sense {
                pos,
                gloss,
                domain,
                examples,
            });
        }

        let mut st = conn.prepare_cached(
            "SELECT target, type FROM relation WHERE entry_id = ?1 ORDER BY rowid",
        )?;
        let relations = st
            .query_map([id], |r| {
                Ok(Relation {
                    target: r.get(0)?,
                    kind: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let lemmas = if lang == Lang::En {
            Self::inflection_lemmas(&conn, &normalize(&headword))?
                .into_iter()
                .map(|(l, _)| l)
                .filter(|l| normalize(l) != normalize(&headword))
                .collect()
        } else {
            Vec::new()
        };
        Ok(Some(Entry {
            id,
            headword,
            lang,
            ipa_us,
            ipa_uk,
            frequency,
            senses,
            relations,
            lemmas,
        }))
    }
}

#[cfg(test)]
mod tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn dictionary_is_send_sync() {
        assert_send_sync::<super::Dictionary>();
    }

    #[test]
    fn truncate() {
        assert_eq!(super::truncate_chars("abcdef", 3), "abc…");
        assert_eq!(super::truncate_chars("ab", 3), "ab");
    }
}
