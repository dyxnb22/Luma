use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use luma_domain::MAX_QUICKLINKS;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

/// Max UTF-8 bytes for a quicklink URL on upsert (personal-use guardrail).
pub const MAX_URL_BYTES: usize = 64 * 1024;

/// Max UTF-8 bytes for a quicklink trigger on upsert.
pub const MAX_TRIGGER_BYTES: usize = 1024;

#[derive(Debug, Error)]
pub enum QuicklinksStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuicklinkRow {
    pub trigger: String,
    pub url: String,
}

pub struct QuicklinksStore {
    path: PathBuf,
}

impl QuicklinksStore {
    pub fn luma_next_default() -> Result<Self, QuicklinksStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("quicklinks.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, QuicklinksStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, QuicklinksStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), QuicklinksStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS quicklinks (trigger TEXT PRIMARY KEY, url TEXT NOT NULL);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<QuicklinkRow>, QuicklinksStoreError> {
        let conn = self.connect()?;
        let mut statement =
            conn.prepare("SELECT trigger, url FROM quicklinks ORDER BY trigger LIMIT ?1")?;
        let rows: Vec<QuicklinkRow> = statement
            .query_map(params![(MAX_QUICKLINKS + 1) as i64], |row| {
                Ok(QuicklinkRow {
                    trigger: row.get(0)?,
                    url: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        if rows.len() > MAX_QUICKLINKS {
            return Err(QuicklinksStoreError::Msg(format!(
                "quicklinks capacity exceeded ({MAX_QUICKLINKS}); delete an entry before continuing"
            )));
        }
        Ok(rows)
    }

    pub fn upsert(&self, trigger: &str, url: &str) -> Result<(), QuicklinksStoreError> {
        if trigger.len() > MAX_TRIGGER_BYTES {
            return Err(QuicklinksStoreError::Msg(format!(
                "quicklink trigger exceeds max size ({MAX_TRIGGER_BYTES} bytes)"
            )));
        }
        if url.len() > MAX_URL_BYTES {
            return Err(QuicklinksStoreError::Msg(format!(
                "quicklink url exceeds max size ({MAX_URL_BYTES} bytes)"
            )));
        }
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM quicklinks WHERE trigger = ?1)",
            params![trigger],
            |row| row.get(0),
        )?;
        if !exists {
            let count: i64 =
                tx.query_row("SELECT COUNT(*) FROM quicklinks", [], |row| row.get(0))?;
            if count >= MAX_QUICKLINKS as i64 {
                return Err(QuicklinksStoreError::Msg(format!(
                    "quicklinks capacity reached ({MAX_QUICKLINKS}); delete an entry before adding another"
                )));
            }
        }
        tx.execute(
            "INSERT INTO quicklinks (trigger, url) VALUES (?1, ?2)
             ON CONFLICT(trigger) DO UPDATE SET url = excluded.url",
            params![trigger, url],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete(&self, trigger: &str) -> Result<(), QuicklinksStoreError> {
        self.connect()?.execute(
            "DELETE FROM quicklinks WHERE trigger = ?1",
            params![trigger],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn upsert_list_delete() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        store.upsert("gh", "https://github.com/{query}").unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        store.delete("gh").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn upsert_rejects_oversized_url() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        let huge = "x".repeat(MAX_URL_BYTES + 1);
        let err = store.upsert("big", &huge).unwrap_err().to_string();
        assert!(err.contains("max size"), "{err}");
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn capacity_rejects_new_rows_but_allows_updates() {
        let dir = tempdir().unwrap();
        let store = QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap();
        let conn = store.connect().unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        for i in 0..MAX_QUICKLINKS {
            tx.execute(
                "INSERT INTO quicklinks (trigger, url) VALUES (?1, 'https://example.com')",
                params![format!("q{i:04}")],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        store.upsert("q0000", "https://updated.example").unwrap();
        let err = store
            .upsert("overflow", "https://example.com")
            .unwrap_err()
            .to_string();
        assert!(err.contains("capacity reached"), "{err}");
        assert_eq!(store.list().unwrap().len(), MAX_QUICKLINKS);
    }
}
