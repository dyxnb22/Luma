use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
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
        let mut statement = conn.prepare("SELECT trigger, url FROM quicklinks ORDER BY trigger")?;
        let rows = statement
            .query_map([], |row| {
                Ok(QuicklinkRow {
                    trigger: row.get(0)?,
                    url: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
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
        self.connect()?.execute(
            "INSERT INTO quicklinks (trigger, url) VALUES (?1, ?2)
             ON CONFLICT(trigger) DO UPDATE SET url = excluded.url",
            params![trigger, url],
        )?;
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
}
