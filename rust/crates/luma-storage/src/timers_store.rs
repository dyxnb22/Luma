use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TimersStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Optimistic-lock miss: row missing or `updated_at_ms` no longer matches.
    #[error("timer update conflict (stale)")]
    Conflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerRow {
    pub id: String,
    pub name: String,
    /// `stopwatch` | `countdown`
    pub kind: String,
    /// `idle` | `running` | `paused` | `completed`
    pub state: String,
    /// Target duration for countdown; `None` for stopwatch.
    pub duration_ms: Option<i64>,
    pub accumulated_ms: i64,
    /// Wall-clock unix ms when the current running segment started.
    pub started_at_ms: Option<i64>,
    pub alerted: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub struct TimersStore {
    path: PathBuf,
}

impl TimersStore {
    pub fn luma_next_default() -> Result<Self, TimersStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("timers.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, TimersStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, TimersStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), TimersStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.connect()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS timers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                state TEXT NOT NULL,
                duration_ms INTEGER,
                accumulated_ms INTEGER NOT NULL DEFAULT 0,
                started_at_ms INTEGER,
                alerted INTEGER NOT NULL DEFAULT 0,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_timers_updated ON timers(updated_at_ms DESC);",
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<TimerRow>, TimersStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT id, name, kind, state, duration_ms, accumulated_ms, started_at_ms,
                    alerted, created_at_ms, updated_at_ms
             FROM timers
             ORDER BY updated_at_ms DESC, name ASC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(TimerRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    state: row.get(3)?,
                    duration_ms: row.get(4)?,
                    accumulated_ms: row.get(5)?,
                    started_at_ms: row.get(6)?,
                    alerted: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn get(&self, id: &str) -> Result<Option<TimerRow>, TimersStoreError> {
        let conn = self.connect()?;
        let mut statement = conn.prepare(
            "SELECT id, name, kind, state, duration_ms, accumulated_ms, started_at_ms,
                    alerted, created_at_ms, updated_at_ms
             FROM timers WHERE id = ?1",
        )?;
        let row = statement
            .query_row(params![id], |row| {
                Ok(TimerRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    state: row.get(3)?,
                    duration_ms: row.get(4)?,
                    accumulated_ms: row.get(5)?,
                    started_at_ms: row.get(6)?,
                    alerted: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn insert(&self, row: &TimerRow) -> Result<(), TimersStoreError> {
        self.connect()?.execute(
            "INSERT INTO timers (
                id, name, kind, state, duration_ms, accumulated_ms, started_at_ms,
                alerted, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                row.id,
                row.name,
                row.kind,
                row.state,
                row.duration_ms,
                row.accumulated_ms,
                row.started_at_ms,
                if row.alerted { 1 } else { 0 },
                row.created_at_ms,
                row.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    /// Compare-and-swap update: succeeds only when `id` exists and
    /// `updated_at_ms` still equals `expected_updated_at_ms`.
    pub fn update(
        &self,
        row: &TimerRow,
        expected_updated_at_ms: i64,
    ) -> Result<(), TimersStoreError> {
        let n = self.connect()?.execute(
            "UPDATE timers SET
                name = ?2,
                kind = ?3,
                state = ?4,
                duration_ms = ?5,
                accumulated_ms = ?6,
                started_at_ms = ?7,
                alerted = ?8,
                updated_at_ms = ?9
             WHERE id = ?1 AND updated_at_ms = ?10",
            params![
                row.id,
                row.name,
                row.kind,
                row.state,
                row.duration_ms,
                row.accumulated_ms,
                row.started_at_ms,
                if row.alerted { 1 } else { 0 },
                row.updated_at_ms,
                expected_updated_at_ms,
            ],
        )?;
        if n == 0 {
            return Err(TimersStoreError::Conflict);
        }
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), TimersStoreError> {
        self.connect()?
            .execute("DELETE FROM timers WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn new_id() -> String {
        format!("tm-{}", Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample(now: i64) -> TimerRow {
        TimerRow {
            id: TimersStore::new_id(),
            name: "Focus".into(),
            kind: "countdown".into(),
            state: "running".into(),
            duration_ms: Some(25 * 60 * 1000),
            accumulated_ms: 0,
            started_at_ms: Some(now),
            alerted: false,
            created_at_ms: now,
            updated_at_ms: now,
        }
    }

    #[test]
    fn round_trip_insert_list_update_delete() {
        let dir = tempdir().unwrap();
        let store = TimersStore::with_path(dir.path().join("timers.sqlite")).unwrap();
        let mut row = sample(1_700_000_000_000);
        store.insert(&row).unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Focus");

        let expected = row.updated_at_ms;
        row.state = "paused".into();
        row.accumulated_ms = 12_000;
        row.started_at_ms = None;
        row.updated_at_ms = 1_700_000_001_000;
        store.update(&row, expected).unwrap();
        let got = store.get(&row.id).unwrap().unwrap();
        assert_eq!(got.state, "paused");
        assert_eq!(got.accumulated_ms, 12_000);
        assert!(got.started_at_ms.is_none());

        store.delete(&row.id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn update_rejects_stale_updated_at() {
        let dir = tempdir().unwrap();
        let store = TimersStore::with_path(dir.path().join("timers.sqlite")).unwrap();
        let row = sample(1_700_000_000_000);
        store.insert(&row).unwrap();

        let mut winner = row.clone();
        winner.state = "paused".into();
        winner.updated_at_ms = row.updated_at_ms + 1_000;
        store.update(&winner, row.updated_at_ms).unwrap();

        let mut stale = row.clone();
        stale.state = "completed".into();
        stale.alerted = true;
        stale.updated_at_ms = row.updated_at_ms + 2_000;
        let err = store.update(&stale, row.updated_at_ms).unwrap_err();
        assert!(
            matches!(err, TimersStoreError::Conflict),
            "expected Conflict, got {err:?}"
        );

        let got = store.get(&row.id).unwrap().unwrap();
        assert_eq!(got.state, "paused");
        assert!(!got.alerted);
        assert_eq!(got.updated_at_ms, winner.updated_at_ms);
    }

    #[test]
    fn update_rejects_missing_id_as_conflict() {
        let dir = tempdir().unwrap();
        let store = TimersStore::with_path(dir.path().join("timers.sqlite")).unwrap();
        let row = sample(1_700_000_000_000);
        let err = store.update(&row, row.updated_at_ms).unwrap_err();
        assert!(matches!(err, TimersStoreError::Conflict));
    }
}
