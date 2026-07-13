//! Adapters that implement application ports using `luma-storage` concrete types.

use crate::ports::{ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError};
use async_trait::async_trait;
use luma_storage::ClipboardStore;
use std::sync::Arc;

pub struct SqliteClipboardHistory {
    store: Arc<ClipboardStore>,
}

impl SqliteClipboardHistory {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self { store }
    }
}

fn map_row(row: luma_storage::ClipboardRow) -> ClipboardEntry {
    ClipboardEntry {
        id: row.id,
        text: row.text,
        pinned: row.pinned,
        created_at: row.created_at,
    }
}

#[async_trait]
impl ClipboardHistoryRepository for SqliteClipboardHistory {
    fn list_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError> {
        self.store
            .list_page(offset, limit)
            .map(|rows| rows.into_iter().map(map_row).collect())
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn latest_by_created(&self) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
        self.store
            .latest_by_created()
            .map(|opt| opt.map(map_row))
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn purge_older_than_days(&self, days: u32) -> Result<usize, ClipboardRepoError> {
        self.store
            .purge_older_than_days(days)
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn insert(&self, text: &str, pinned: bool) -> Result<i64, ClipboardRepoError> {
        self.store
            .insert(text, pinned)
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn get(&self, id: i64) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
        self.store
            .get(id)
            .map(|opt| opt.map(map_row))
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn delete(&self, id: i64) -> Result<(), ClipboardRepoError> {
        self.store
            .delete(id)
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), ClipboardRepoError> {
        self.store
            .set_pinned(id, pinned)
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }

    fn clear_unpinned(&self) -> Result<usize, ClipboardRepoError> {
        self.store
            .clear_unpinned()
            .map_err(|e| ClipboardRepoError::msg(e.to_string()))
    }
}
