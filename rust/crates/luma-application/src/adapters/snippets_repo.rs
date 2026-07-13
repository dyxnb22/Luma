use crate::ports::{SnippetEntry, SnippetsRepoError, SnippetsRepository};
use async_trait::async_trait;
use luma_storage::SnippetsStore;
use std::sync::Arc;

pub struct SqliteSnippetsRepository {
    store: Arc<SnippetsStore>,
}

impl SqliteSnippetsRepository {
    pub fn new(store: Arc<SnippetsStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl SnippetsRepository for SqliteSnippetsRepository {
    fn list(&self) -> Result<Vec<SnippetEntry>, SnippetsRepoError> {
        self.store
            .list()
            .map(|rows| {
                rows.into_iter()
                    .map(|r| SnippetEntry {
                        trigger: r.trigger,
                        body: r.body,
                    })
                    .collect()
            })
            .map_err(|e| SnippetsRepoError::msg(e.to_string()))
    }

    fn get(&self, trigger: &str) -> Result<Option<SnippetEntry>, SnippetsRepoError> {
        self.store
            .get(trigger)
            .map(|opt| {
                opt.map(|r| SnippetEntry {
                    trigger: r.trigger,
                    body: r.body,
                })
            })
            .map_err(|e| SnippetsRepoError::msg(e.to_string()))
    }

    fn upsert(&self, trigger: &str, body: &str) -> Result<(), SnippetsRepoError> {
        self.store
            .upsert(trigger, body)
            .map_err(|e| SnippetsRepoError::msg(e.to_string()))
    }

    fn delete(&self, trigger: &str) -> Result<(), SnippetsRepoError> {
        self.store
            .delete(trigger)
            .map_err(|e| SnippetsRepoError::msg(e.to_string()))
    }
}
