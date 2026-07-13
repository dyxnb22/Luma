use crate::ports::{QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository};
use async_trait::async_trait;
use luma_storage::QuicklinksStore;
use std::sync::Arc;

pub struct SqliteQuicklinksRepository {
    store: Arc<QuicklinksStore>,
}

impl SqliteQuicklinksRepository {
    pub fn new(store: Arc<QuicklinksStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl QuicklinksRepository for SqliteQuicklinksRepository {
    fn list(&self) -> Result<Vec<QuicklinkEntry>, QuicklinksRepoError> {
        self.store
            .list()
            .map(|rows| {
                rows.into_iter()
                    .map(|r| QuicklinkEntry {
                        trigger: r.trigger,
                        url: r.url,
                    })
                    .collect()
            })
            .map_err(|e| QuicklinksRepoError::msg(e.to_string()))
    }

    fn upsert(&self, trigger: &str, url: &str) -> Result<(), QuicklinksRepoError> {
        self.store
            .upsert(trigger, url)
            .map_err(|e| QuicklinksRepoError::msg(e.to_string()))
    }

    fn delete(&self, trigger: &str) -> Result<(), QuicklinksRepoError> {
        self.store
            .delete(trigger)
            .map_err(|e| QuicklinksRepoError::msg(e.to_string()))
    }
}
