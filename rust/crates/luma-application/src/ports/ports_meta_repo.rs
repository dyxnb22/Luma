//! Local metadata for named / favorite listening ports.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortMeta {
    pub port: u16,
    pub display_name: Option<String>,
    pub favorite: bool,
    pub last_seen_at: Option<String>,
    pub kill_count: i64,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct PortsMetaRepoError(pub String);

impl PortsMetaRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait PortsMetaRepository: Send + Sync {
    fn list(&self) -> Result<Vec<PortMeta>, PortsMetaRepoError>;
    fn get(&self, port: u16) -> Result<Option<PortMeta>, PortsMetaRepoError>;
    fn set_display_name(
        &self,
        port: u16,
        display_name: Option<&str>,
    ) -> Result<(), PortsMetaRepoError>;
    fn set_favorite(&self, port: u16, favorite: bool) -> Result<(), PortsMetaRepoError>;
    fn record_seen(&self, port: u16, seen_at: &str) -> Result<(), PortsMetaRepoError>;
    fn record_kill(&self, port: u16, killed_at: &str) -> Result<(), PortsMetaRepoError>;
    fn delete(&self, port: u16) -> Result<(), PortsMetaRepoError>;
}
