use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

/// Max UTF-8 bytes for a snippet body on upsert (personal-use guardrail).
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Max UTF-8 bytes for a snippet trigger on upsert.
pub const MAX_TRIGGER_BYTES: usize = 1024;

#[derive(Debug, Error)]
pub enum SnippetsStoreError {
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
pub struct SnippetRow {
    pub trigger: String,
    pub body: String,
}

pub struct SnippetsStore {
    path: PathBuf,
}

impl SnippetsStore {
    pub fn luma_next_default() -> Result<Self, SnippetsStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("snippets.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, SnippetsStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, SnippetsStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), SnippetsStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS snippets (trigger TEXT PRIMARY KEY, body TEXT NOT NULL);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<SnippetRow>, SnippetsStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare("SELECT trigger, body FROM snippets ORDER BY trigger")?;
        let rows = statement
            .query_map([], |row| {
                Ok(SnippetRow {
                    trigger: row.get(0)?,
                    body: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get(&self, trigger: &str) -> Result<Option<SnippetRow>, SnippetsStoreError> {
        let conn = self.connect()?;
        let mut statement =
            conn.prepare("SELECT trigger, body FROM snippets WHERE trigger = ?1")?;
        let row = statement
            .query_map(params![trigger], |row| {
                Ok(SnippetRow {
                    trigger: row.get(0)?,
                    body: row.get(1)?,
                })
            })?
            .next()
            .transpose()?;
        Ok(row)
    }

    pub fn upsert(&self, trigger: &str, body: &str) -> Result<(), SnippetsStoreError> {
        if trigger.len() > MAX_TRIGGER_BYTES {
            return Err(SnippetsStoreError::Msg(format!(
                "snippet trigger exceeds max size ({MAX_TRIGGER_BYTES} bytes)"
            )));
        }
        if body.len() > MAX_BODY_BYTES {
            return Err(SnippetsStoreError::Msg(format!(
                "snippet body exceeds max size ({MAX_BODY_BYTES} bytes)"
            )));
        }
        self.connect()?.execute(
            "INSERT INTO snippets (trigger, body) VALUES (?1, ?2)
             ON CONFLICT(trigger) DO UPDATE SET body = excluded.body",
            params![trigger, body],
        )?;
        Ok(())
    }

    pub fn delete(&self, trigger: &str) -> Result<(), SnippetsStoreError> {
        self.connect()?
            .execute("DELETE FROM snippets WHERE trigger = ?1", params![trigger])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn upsert_get_delete() {
        let dir = tempdir().unwrap();
        let store = SnippetsStore::with_path(dir.path().join("sn.sqlite")).unwrap();
        store.upsert(";sig", "Thanks,\nMe").unwrap();
        assert_eq!(store.get(";sig").unwrap().unwrap().body, "Thanks,\nMe");
        store.delete(";sig").unwrap();
        assert!(store.get(";sig").unwrap().is_none());
    }

    #[test]
    fn upsert_rejects_oversized_body() {
        let dir = tempdir().unwrap();
        let store = SnippetsStore::with_path(dir.path().join("sn.sqlite")).unwrap();
        let huge = "x".repeat(MAX_BODY_BYTES + 1);
        let err = store.upsert(";big", &huge).unwrap_err().to_string();
        assert!(err.contains("max size"), "{err}");
        assert!(store.list().unwrap().is_empty());
    }
}
