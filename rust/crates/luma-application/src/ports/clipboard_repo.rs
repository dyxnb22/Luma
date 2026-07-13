use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub id: i64,
    pub text: String,
    pub pinned: bool,
    pub created_at: i64,
}

#[derive(Debug, Error)]
pub enum ClipboardRepoError {
    #[error("{0}")]
    Message(String),
}

impl ClipboardRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

#[async_trait]
pub trait ClipboardHistoryRepository: Send + Sync {
    fn list_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError>;
    fn latest_by_created(&self) -> Result<Option<ClipboardEntry>, ClipboardRepoError>;
    fn purge_older_than_days(&self, days: u32) -> Result<usize, ClipboardRepoError>;
    fn insert(&self, text: &str, pinned: bool) -> Result<i64, ClipboardRepoError>;
    fn get(&self, id: i64) -> Result<Option<ClipboardEntry>, ClipboardRepoError>;
    fn delete(&self, id: i64) -> Result<(), ClipboardRepoError>;
    fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), ClipboardRepoError>;
    /// Atomically delete all unpinned rows; returns how many were removed.
    fn clear_unpinned(&self) -> Result<usize, ClipboardRepoError>;
}

/// Privacy heuristic shared by clipboard capture (not a substitute for suppression leases).
pub fn looks_secret(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("password")
        || lower.contains("secret")
        || lower.contains("api_key")
        || lower.contains("-----begin ")
        || text.len() > 20_000
}
