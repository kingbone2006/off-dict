//! `user.sqlite` (đọc/ghi, WAL): lịch sử, yêu thích, cài đặt.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::normalize::normalize;
use crate::{Lang, UserError};

/// Migration đánh số, chạy theo `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

/// Một dòng lịch sử / yêu thích.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryItem {
    /// Từ.
    pub headword: String,
    /// Ngôn ngữ.
    pub lang: Lang,
    /// Thời điểm (giây UNIX).
    pub at: i64,
}

/// Kho dữ liệu người dùng. `Send + Sync`.
pub struct UserStore {
    conn: Mutex<Connection>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn row_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryItem> {
    let lang: String = r.get(1)?;
    Ok(HistoryItem {
        headword: r.get(0)?,
        lang: Lang::parse(&lang).unwrap_or(Lang::En),
        at: r.get(2)?,
    })
}

impl UserStore {
    /// Mở (hoặc tạo) `user.sqlite`, tự chạy migration.
    pub fn open(path: &Path) -> Result<Self, UserError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    /// Kho trong bộ nhớ (cho test / khi không ghi được đĩa).
    pub fn open_in_memory() -> Result<Self, UserError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, UserError> {
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version.max(0) as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>, UserError> {
        self.conn.lock().map_err(|_| UserError::Poisoned)
    }

    /// Đọc cài đặt.
    pub fn setting(&self, key: &str) -> Result<Option<String>, UserError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached("SELECT value FROM settings WHERE key = ?1")?;
        Ok(st.query_row([key], |r| r.get(0)).optional()?)
    }

    /// Ghi cài đặt.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), UserError> {
        self.conn()?.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    /// Xoá toàn bộ dữ liệu người dùng.
    pub fn clear_all(&self) -> Result<(), UserError> {
        self.conn()?.execute_batch(
            "DELETE FROM history; DELETE FROM favorite; DELETE FROM srs_card; DELETE FROM settings;",
        )?;
        Ok(())
    }
}

impl UserStore {
    /// Ghi một lần tra.
    pub fn add_history(&self, headword: &str, lang: Lang) -> Result<(), UserError> {
        if normalize(headword).is_empty() {
            return Ok(());
        }
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "INSERT INTO history(headword, lang, looked_up_at) VALUES (?1, ?2, ?3)",
        )?;
        st.execute(params![headword.trim(), lang.as_str(), now()])?;
        Ok(())
    }

    /// Lịch sử gần nhất (mỗi từ một dòng), mới nhất trước.
    pub fn recent_history(&self, limit: usize) -> Result<Vec<HistoryItem>, UserError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "SELECT headword, lang, MAX(looked_up_at) AS t, MAX(rowid) AS r FROM history
             GROUP BY lang, headword ORDER BY t DESC, r DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit as i64], row_item)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Tìm trong lịch sử theo chuỗi con.
    pub fn search_history(&self, text: &str, limit: usize) -> Result<Vec<HistoryItem>, UserError> {
        let pat = format!("%{}%", normalize(text).replace(['%', '_'], ""));
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "SELECT headword, lang, MAX(looked_up_at) AS t, MAX(rowid) AS r FROM history
             WHERE lower(headword) LIKE ?1 GROUP BY lang, headword
             ORDER BY t DESC, r DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![pat, limit as i64], row_item)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Xoá toàn bộ lịch sử.
    pub fn clear_history(&self) -> Result<(), UserError> {
        self.conn()?.execute("DELETE FROM history", [])?;
        Ok(())
    }

    /// Xoá một từ khỏi lịch sử.
    pub fn remove_history(&self, headword: &str, lang: Lang) -> Result<(), UserError> {
        self.conn()?.execute(
            "DELETE FROM history WHERE headword = ?1 AND lang = ?2",
            params![headword, lang.as_str()],
        )?;
        Ok(())
    }

    /// Bật/tắt yêu thích. Trả về `true` nếu sau thao tác từ đang được yêu thích.
    pub fn toggle_favorite(&self, headword: &str, lang: Lang) -> Result<bool, UserError> {
        let conn = self.conn()?;
        let removed = conn.execute(
            "DELETE FROM favorite WHERE headword = ?1 AND lang = ?2",
            params![headword, lang.as_str()],
        )?;
        if removed > 0 {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO favorite(headword, lang, folder, note, created_at)
             VALUES (?1, ?2, '', '', ?3)",
            params![headword, lang.as_str(), now()],
        )?;
        Ok(true)
    }

    /// Từ có đang được yêu thích không.
    pub fn is_favorite(&self, headword: &str, lang: Lang) -> Result<bool, UserError> {
        let conn = self.conn()?;
        let mut st =
            conn.prepare_cached("SELECT 1 FROM favorite WHERE headword = ?1 AND lang = ?2")?;
        let hit = st
            .query_row(params![headword, lang.as_str()], |_| Ok(()))
            .optional()?;
        Ok(hit.is_some())
    }

    /// Đặt thư mục (nhóm) cho từ yêu thích.
    pub fn set_favorite_folder(
        &self,
        headword: &str,
        lang: Lang,
        folder: &str,
    ) -> Result<(), UserError> {
        self.conn()?.execute(
            "UPDATE favorite SET folder = ?3 WHERE headword = ?1 AND lang = ?2",
            params![headword, lang.as_str(), folder.trim()],
        )?;
        Ok(())
    }

    /// Danh sách yêu thích (mới nhất trước), lọc theo thư mục nếu có.
    pub fn favorites(&self, folder: Option<&str>) -> Result<Vec<HistoryItem>, UserError> {
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "SELECT headword, lang, created_at FROM favorite
             WHERE ?1 IS NULL OR folder = ?1 ORDER BY created_at DESC, rowid DESC",
        )?;
        let rows = st.query_map([folder], row_item)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_and_favorites() -> Result<(), UserError> {
        let s = UserStore::open_in_memory()?;
        s.add_history("run", Lang::En)?;
        s.add_history("m\u{e8}o", Lang::Vi)?;
        s.add_history("run", Lang::En)?;
        s.add_history("   ", Lang::En)?;
        let h = s.recent_history(10)?;
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].headword, "run");
        assert_eq!(s.search_history("m\u{e8}", 10)?.len(), 1);
        assert!(s.toggle_favorite("cat", Lang::En)?);
        assert!(s.is_favorite("cat", Lang::En)?);
        s.set_favorite_folder("cat", Lang::En, "animals")?;
        assert_eq!(s.favorites(Some("animals"))?.len(), 1);
        assert!(!s.toggle_favorite("cat", Lang::En)?);
        assert!(s.favorites(None)?.is_empty());
        s.set_setting("theme", "dark")?;
        s.set_setting("theme", "light")?;
        assert_eq!(s.setting("theme")?.as_deref(), Some("light"));
        s.clear_history()?;
        assert!(s.recent_history(10)?.is_empty());
        Ok(())
    }

    #[test]
    fn reopen_runs_migrations_once() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path().join("sub").join("user.sqlite");
        UserStore::open(&p)?.add_history("go", Lang::En)?;
        let s = UserStore::open(&p)?;
        assert_eq!(s.recent_history(5)?.len(), 1);
        Ok(())
    }
}
