use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuicklinksStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
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
