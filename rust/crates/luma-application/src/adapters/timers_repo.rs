use crate::ports::{TimerEntry, TimersRepoError, TimersRepository};
use luma_storage::{TimerRow, TimersStore};
use std::sync::Arc;

pub struct SqliteTimersRepository {
    store: Arc<TimersStore>,
}

impl SqliteTimersRepository {
    pub fn new(store: Arc<TimersStore>) -> Self {
        Self { store }
    }
}

fn from_row(row: TimerRow) -> TimerEntry {
    TimerEntry {
        id: row.id,
        name: row.name,
        kind: row.kind,
        state: row.state,
        duration_ms: row.duration_ms,
        accumulated_ms: row.accumulated_ms,
        started_at_ms: row.started_at_ms,
        alerted: row.alerted,
        created_at_ms: row.created_at_ms,
        updated_at_ms: row.updated_at_ms,
    }
}

fn to_row(entry: &TimerEntry) -> TimerRow {
    TimerRow {
        id: entry.id.clone(),
        name: entry.name.clone(),
        kind: entry.kind.clone(),
        state: entry.state.clone(),
        duration_ms: entry.duration_ms,
        accumulated_ms: entry.accumulated_ms,
        started_at_ms: entry.started_at_ms,
        alerted: entry.alerted,
        created_at_ms: entry.created_at_ms,
        updated_at_ms: entry.updated_at_ms,
    }
}

impl TimersRepository for SqliteTimersRepository {
    fn list(&self) -> Result<Vec<TimerEntry>, TimersRepoError> {
        self.store
            .list()
            .map(|rows| rows.into_iter().map(from_row).collect())
            .map_err(|e| TimersRepoError::msg(e.to_string()))
    }

    fn get(&self, id: &str) -> Result<Option<TimerEntry>, TimersRepoError> {
        self.store
            .get(id)
            .map(|opt| opt.map(from_row))
            .map_err(|e| TimersRepoError::msg(e.to_string()))
    }

    fn insert(&self, entry: &TimerEntry) -> Result<(), TimersRepoError> {
        self.store
            .insert(&to_row(entry))
            .map_err(|e| TimersRepoError::msg(e.to_string()))
    }

    fn update(&self, entry: &TimerEntry) -> Result<(), TimersRepoError> {
        self.store
            .update(&to_row(entry))
            .map_err(|e| TimersRepoError::msg(e.to_string()))
    }

    fn delete(&self, id: &str) -> Result<(), TimersRepoError> {
        self.store
            .delete(id)
            .map_err(|e| TimersRepoError::msg(e.to_string()))
    }

    fn new_id(&self) -> String {
        TimersStore::new_id()
    }
}
