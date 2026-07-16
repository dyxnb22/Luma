use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshHostMeta {
    pub alias: String,
    pub display_name: Option<String>,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub last_connected_at: Option<String>,
    pub connection_count: i64,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct SshMetaRepoError(pub String);

impl SshMetaRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait SshMetaRepository: Send + Sync {
    fn list(&self) -> Result<Vec<SshHostMeta>, SshMetaRepoError>;
    fn get(&self, alias: &str) -> Result<Option<SshHostMeta>, SshMetaRepoError>;
    fn set_favorite(&self, alias: &str, favorite: bool) -> Result<(), SshMetaRepoError>;
    fn set_display_name(
        &self,
        alias: &str,
        display_name: Option<&str>,
    ) -> Result<(), SshMetaRepoError>;
    fn record_connection(&self, alias: &str, connected_at: &str) -> Result<(), SshMetaRepoError>;
    fn delete(&self, alias: &str) -> Result<(), SshMetaRepoError>;
}
