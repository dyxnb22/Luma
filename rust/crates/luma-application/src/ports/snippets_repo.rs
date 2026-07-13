use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetEntry {
    pub trigger: String,
    pub body: String,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct SnippetsRepoError(pub String);

impl SnippetsRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait SnippetsRepository: Send + Sync {
    fn list(&self) -> Result<Vec<SnippetEntry>, SnippetsRepoError>;
    fn get(&self, trigger: &str) -> Result<Option<SnippetEntry>, SnippetsRepoError>;
    fn upsert(&self, trigger: &str, body: &str) -> Result<(), SnippetsRepoError>;
    fn delete(&self, trigger: &str) -> Result<(), SnippetsRepoError>;
}
