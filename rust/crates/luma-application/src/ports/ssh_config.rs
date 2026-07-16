use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedSshHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub connect_timeout: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SshConfigState {
    Found,
    NotConfigured,
    Unavailable(String),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct SshConfigError(pub String);

impl SshConfigError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Sanitize identity file path for display (basename only, no file reads).
pub fn sanitize_identity_display(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "-".into();
    }
    if trimmed.contains("-----BEGIN") {
        return "[redacted]".into();
    }
    std::path::Path::new(trimmed)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(trimmed)
        .to_string()
}

/// Build `user@host:port` subtitle from resolved host.
pub fn format_connection_subtitle(host: &ResolvedSshHost) -> String {
    let user = host.user.as_deref().unwrap_or("-");
    let hostname = host.hostname.as_deref().unwrap_or("-");
    let port = host.port.unwrap_or(22);
    format!("{user}@{hostname}:{port}")
}

#[async_trait]
pub trait SshConfigPort: Send + Sync {
    fn config_state(&self) -> SshConfigState;
    fn list_aliases(&self) -> Result<Vec<String>, SshConfigError>;
    fn resolve(&self, alias: &str) -> Result<ResolvedSshHost, SshConfigError>;
    fn ssh_available(&self) -> bool;
    fn sftp_available(&self) -> bool;
}
