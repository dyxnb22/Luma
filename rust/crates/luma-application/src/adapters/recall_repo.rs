use crate::ports::{RecallObject, RecallRepoError, RecallRepository};
use async_trait::async_trait;
use luma_storage::{RecallRow, RecallStore};
use std::sync::Arc;

pub struct SqliteRecallRepository {
    store: Arc<RecallStore>,
}

impl SqliteRecallRepository {
    pub fn new(store: Arc<RecallStore>) -> Self {
        Self { store }
    }
}

fn from_row(row: RecallRow) -> RecallObject {
    RecallObject {
        object_id: row.object_id,
        module_id: row.module_id,
        kind: row.kind,
        primary_action: row.primary_action,
        title: row.title,
        project_path: row.project_path,
        use_count: row.use_count,
        last_used_at: row.last_used_at,
    }
}

#[async_trait]
impl RecallRepository for SqliteRecallRepository {
    fn record_success(&self, object: RecallObject) -> Result<(), RecallRepoError> {
        self.store
            .record_success(&RecallRow {
                object_id: object.object_id,
                module_id: object.module_id,
                kind: object.kind,
                primary_action: object.primary_action,
                title: object.title,
                project_path: object.project_path,
                use_count: object.use_count,
                last_used_at: object.last_used_at,
            })
            .map_err(|err| RecallRepoError(err.to_string()))
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<RecallObject>, RecallRepoError> {
        self.store
            .list_recent(limit)
            .map(|items| items.into_iter().map(from_row).collect())
            .map_err(|err| RecallRepoError(err.to_string()))
    }
}
