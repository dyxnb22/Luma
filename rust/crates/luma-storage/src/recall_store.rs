use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

/// Recall metadata is intentionally small. It never stores result payloads, search text,
/// record notes, clipboard/snippet bodies, credentials, or preview text.
pub const MAX_RECALL_OBJECTS: usize = 1_000;
const RECALL_SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Error)]
pub enum RecallStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecallRow {
    pub object_id: String,
    pub module_id: String,
    pub kind: String,
    pub primary_action: String,
    /// A bounded, non-body display label. Clipboard and SSH rows deliberately use generic
    /// labels so the Recall DB does not duplicate sensitive source data.
    pub title: String,
    pub project_path: Option<String>,
    pub use_count: i64,
    pub last_used_at: i64,
}

pub struct RecallStore {
    path: PathBuf,
}

impl RecallStore {
    pub fn luma_next_default() -> Result<Self, RecallStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("recall.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, RecallStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, RecallStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), RecallStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.connect()?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version == 0 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS recall_objects (
                    object_id TEXT PRIMARY KEY,
                    module_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    primary_action TEXT NOT NULL,
                    title TEXT NOT NULL,
                    project_path TEXT,
                    use_count INTEGER NOT NULL DEFAULT 0,
                    last_used_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS recall_objects_recent
                    ON recall_objects(last_used_at DESC, use_count DESC);
                PRAGMA user_version = 2;",
            )?;
        } else if version == 1 {
            conn.execute_batch(
                "ALTER TABLE recall_objects ADD COLUMN primary_action TEXT NOT NULL DEFAULT 'open';
                 PRAGMA user_version = 2;",
            )?;
        }
        debug_assert!(version <= RECALL_SCHEMA_VERSION);
        Ok(())
    }

    pub fn record_success(&self, row: &RecallRow) -> Result<(), RecallStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO recall_objects
                (object_id, module_id, kind, primary_action, title, project_path, use_count, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)
             ON CONFLICT(object_id) DO UPDATE SET
                module_id = excluded.module_id,
                kind = excluded.kind,
                primary_action = excluded.primary_action,
                title = excluded.title,
                project_path = excluded.project_path,
                use_count = recall_objects.use_count + 1,
                last_used_at = excluded.last_used_at",
            params![
                row.object_id,
                row.module_id,
                row.kind,
                row.primary_action,
                row.title,
                row.project_path,
                row.last_used_at,
            ],
        )?;
        // Keep the most useful rows. Deletion is deterministic, and an existing row can always
        // be refreshed at capacity.
        tx.execute(
            "DELETE FROM recall_objects
             WHERE object_id NOT IN (
                SELECT object_id FROM recall_objects
                ORDER BY last_used_at DESC, use_count DESC, object_id ASC
                LIMIT ?1
             )",
            params![MAX_RECALL_OBJECTS as i64],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<RecallRow>, RecallStoreError> {
        let capped = limit.min(MAX_RECALL_OBJECTS) as i64;
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT object_id, module_id, kind, primary_action, title, project_path, use_count, last_used_at
             FROM recall_objects
             ORDER BY last_used_at DESC, use_count DESC, object_id ASC
             LIMIT ?1",
        )?;
        let rows = statement
            .query_map(params![capped], |row| {
                Ok(RecallRow {
                    object_id: row.get(0)?,
                    module_id: row.get(1)?,
                    kind: row.get(2)?,
                    primary_action: row.get(3)?,
                    title: row.get(4)?,
                    project_path: row.get(5)?,
                    use_count: row.get(6)?,
                    last_used_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(RecallStoreError::from)?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn row(id: &str, at: i64) -> RecallRow {
        RecallRow {
            object_id: id.into(),
            module_id: "luma.projects".into(),
            kind: "project".into(),
            primary_action: "open".into(),
            title: "Project".into(),
            project_path: Some("/tmp/project".into()),
            use_count: 0,
            last_used_at: at,
        }
    }

    #[test]
    fn records_success_without_storing_payloads_and_orders_recent() {
        let dir = tempdir().unwrap();
        let store = RecallStore::with_path(dir.path().join("recall.sqlite")).unwrap();
        store.record_success(&row("proj:a", 10)).unwrap();
        store.record_success(&row("proj:b", 20)).unwrap();
        store.record_success(&row("proj:a", 30)).unwrap();
        let rows = store.list_recent(10).unwrap();
        assert_eq!(rows[0].object_id, "proj:a");
        assert_eq!(rows[0].use_count, 2);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn trims_oldest_rows_at_capacity() {
        let dir = tempdir().unwrap();
        let store = RecallStore::with_path(dir.path().join("recall.sqlite")).unwrap();
        for n in 0..=MAX_RECALL_OBJECTS {
            store
                .record_success(&row(&format!("p:{n}"), n as i64))
                .unwrap();
        }
        let rows = store.list_recent(MAX_RECALL_OBJECTS + 10).unwrap();
        assert_eq!(rows.len(), MAX_RECALL_OBJECTS);
        assert!(!rows.iter().any(|item| item.object_id == "p:0"));
    }
}
