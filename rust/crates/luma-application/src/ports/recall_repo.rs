use async_trait::async_trait;
use thiserror::Error;

pub const MAX_RECALL_TITLE_CHARS: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecallObject {
    pub object_id: String,
    pub module_id: String,
    pub kind: String,
    pub primary_action: String,
    pub title: String,
    pub project_path: Option<String>,
    pub use_count: i64,
    pub last_used_at: i64,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct RecallRepoError(pub String);

#[async_trait]
pub trait RecallRepository: Send + Sync {
    fn record_success(&self, object: RecallObject) -> Result<(), RecallRepoError>;
    fn list_recent(&self, limit: usize) -> Result<Vec<RecallObject>, RecallRepoError>;
}
