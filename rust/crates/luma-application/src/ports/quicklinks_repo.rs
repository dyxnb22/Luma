use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuicklinkEntry {
    pub trigger: String,
    pub url: String,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct QuicklinksRepoError(pub String);

impl QuicklinksRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait QuicklinksRepository: Send + Sync {
    fn list(&self) -> Result<Vec<QuicklinkEntry>, QuicklinksRepoError>;
    fn upsert(&self, trigger: &str, url: &str) -> Result<(), QuicklinksRepoError>;
    fn delete(&self, trigger: &str) -> Result<(), QuicklinksRepoError>;
}
