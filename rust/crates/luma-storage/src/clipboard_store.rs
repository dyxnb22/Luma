//! Clipboard history under LumaNext (SQLite).

use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Soft cap on unpinned history rows. Pinned rows are never evicted and do not
/// count toward this limit. Chosen for personal daily use (keeps DB small).
pub const MAX_UNPINNED_ROWS: usize = 500;

/// Max UTF-8 bytes accepted per clipboard entry on insert.
/// Oversized pasteboard values are rejected (not truncated).
pub const MAX_ENTRY_BYTES: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum ClipboardStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardRow {
    pub id: i64,
    pub text: String,
    pub pinned: bool,
    pub created_at: i64,
}

pub struct ClipboardStore {
    path: PathBuf,
}

impl ClipboardStore {
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn luma_next_default() -> Result<Self, ClipboardStoreError> {
        ensure_luma_next_dirs()?;
        let path = luma_next_support_dir()?.join("clipboard.sqlite");
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    pub fn with_path(path: PathBuf) -> Result<Self, ClipboardStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, ClipboardStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), ClipboardStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_entries(created_at DESC);
            "#,
        )?;
        Ok(())
    }

    pub fn insert(&self, text: &str, pinned: bool) -> Result<i64, ClipboardStoreError> {
        if text.len() > MAX_ENTRY_BYTES {
            return Err(ClipboardStoreError::Msg(format!(
                "clipboard entry exceeds max size ({MAX_ENTRY_BYTES} bytes)"
            )));
        }
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        tx.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES (?1, ?2, ?3)",
            params![text, pinned as i64, now],
        )?;
        let id = tx.last_insert_rowid();
        // Evict oldest unpinned rows when over the soft cap (pinned exempt).
        let unpinned: i64 = tx.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE pinned = 0",
            [],
            |r| r.get(0),
        )?;
        if unpinned > MAX_UNPINNED_ROWS as i64 {
            let excess = unpinned - MAX_UNPINNED_ROWS as i64;
            tx.execute(
                "DELETE FROM clipboard_entries WHERE id IN (
                    SELECT id FROM clipboard_entries WHERE pinned = 0
                    ORDER BY created_at ASC, id ASC
                    LIMIT ?1
                 )",
                params![excess],
            )?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn search(
        &self,
        needle: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardRow>, ClipboardStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, text, pinned, created_at FROM clipboard_entries
             WHERE text LIKE ?1
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?2",
        )?;
        let pattern = format!("%{needle}%");
        let rows = stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(ClipboardRow {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    pinned: row.get::<_, i64>(2)? != 0,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ClipboardRow>, ClipboardStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, text, pinned, created_at FROM clipboard_entries
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(ClipboardRow {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    pinned: row.get::<_, i64>(2)? != 0,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Newest row by insert time, ignoring pin order (for pasteboard change dedupe).
    pub fn latest_by_created(&self) -> Result<Option<ClipboardRow>, ClipboardStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, text, pinned, created_at FROM clipboard_entries
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(ClipboardRow {
                id: row.get(0)?,
                text: row.get(1)?,
                pinned: row.get::<_, i64>(2)? != 0,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Delete unpinned rows older than `days`. Pinned rows are retained.
    pub fn purge_older_than_days(&self, days: u32) -> Result<usize, ClipboardStoreError> {
        let conn = self.connect()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let cutoff = now.saturating_sub(i64::from(days).saturating_mul(86_400));
        let n = conn.execute(
            "DELETE FROM clipboard_entries WHERE pinned = 0 AND created_at < ?1",
            params![cutoff],
        )?;
        Ok(n)
    }

    pub fn count(&self) -> Result<usize, ClipboardStoreError> {
        let conn = self.connect()?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_entries", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn get(&self, id: i64) -> Result<Option<ClipboardRow>, ClipboardStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, text, pinned, created_at FROM clipboard_entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(ClipboardRow {
                id: row.get(0)?,
                text: row.get(1)?,
                pinned: row.get::<_, i64>(2)? != 0,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn delete(&self, id: i64) -> Result<(), ClipboardStoreError> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), ClipboardStoreError> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE clipboard_entries SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    /// Single-statement delete of every unpinned row.
    pub fn clear_unpinned(&self) -> Result<usize, ClipboardStoreError> {
        let conn = self.connect()?;
        let n = conn.execute("DELETE FROM clipboard_entries WHERE pinned = 0", [])?;
        Ok(n)
    }
}

/// Privacy filter shared with the Clipboard module.
pub use luma_domain::looks_secret;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn insert_search_page() {
        let dir = tempdir().unwrap();
        let store = ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap();
        let id = store.insert("invoice 42", false).unwrap();
        assert!(id > 0);
        let hits = store.search("invoice", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(store.count().unwrap(), 1);
        assert!(looks_secret("password=x"));
        assert!(!looks_secret("hello"));
    }

    #[test]
    fn latest_by_created_ignores_pin_order() {
        let dir = tempdir().unwrap();
        let store = ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap();
        let conn = rusqlite::Connection::open(store.path()).unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('old', 0, 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('pinned', 1, 20)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('newest', 0, 30)",
            [],
        )
        .unwrap();
        let latest = store.latest_by_created().unwrap().unwrap();
        assert_eq!(latest.text, "newest");
        let page = store.list_page(0, 1).unwrap();
        assert_eq!(page[0].text, "pinned");
    }

    #[test]
    fn purge_keeps_pinned() {
        let dir = tempdir().unwrap();
        let store = ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap();
        let conn = rusqlite::Connection::open(store.path()).unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('old', 0, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('keep', 1, 1)",
            [],
        )
        .unwrap();
        assert_eq!(store.purge_older_than_days(1).unwrap(), 1);
        assert_eq!(store.count().unwrap(), 1);
        assert_eq!(store.list_page(0, 10).unwrap()[0].text, "keep");
    }

    #[test]
    fn insert_rejects_oversized_entry() {
        let dir = tempdir().unwrap();
        let store = ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap();
        let huge = "x".repeat(MAX_ENTRY_BYTES + 1);
        let err = store.insert(&huge, false).unwrap_err().to_string();
        assert!(err.contains("max size"), "{err}");
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn insert_evicts_oldest_unpinned_over_cap() {
        let dir = tempdir().unwrap();
        let store = ClipboardStore::with_path(dir.path().join("c.sqlite")).unwrap();
        // Seed just over the unpinned cap via direct SQL for speed, then insert once.
        let conn = rusqlite::Connection::open(store.path()).unwrap();
        for i in 0..MAX_UNPINNED_ROWS {
            conn.execute(
                "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES (?1, 0, ?2)",
                params![format!("row-{i}"), i as i64],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO clipboard_entries (text, pinned, created_at) VALUES ('pinned-keep', 1, 0)",
            [],
        )
        .unwrap();
        drop(conn);

        store.insert("newest", false).unwrap();
        let unpinned: i64 = {
            let conn = rusqlite::Connection::open(store.path()).unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM clipboard_entries WHERE pinned = 0",
                [],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(unpinned, MAX_UNPINNED_ROWS as i64);
        assert!(store.search("pinned-keep", 5).unwrap().len() == 1);
        assert!(store.search("newest", 5).unwrap().len() == 1);
        assert!(store.search("row-0", 5).unwrap().is_empty());
    }
}
