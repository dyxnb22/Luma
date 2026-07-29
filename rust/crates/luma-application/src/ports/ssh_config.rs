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

pub const SSH_PASSWORD_ACCOUNT_PREFIX: &str = "ssh-password:";
pub const SSH_ASKPASS_ACCOUNT_ENV: &str = "LUMA_SSH_ASKPASS_ACCOUNT";

pub fn ssh_password_account(alias: &str) -> String {
    format!("{SSH_PASSWORD_ACCOUNT_PREFIX}{alias}")
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

    /// Build argv for an interactive connection. Adapters that enumerate an explicit
    /// non-default config override these so resolution and connection cannot drift.
    fn ssh_invocation_args(&self, alias: &str) -> Vec<String> {
        vec!["--".into(), alias.into()]
    }

    fn sftp_invocation_args(&self, alias: &str) -> Vec<String> {
        vec!["--".into(), alias.into()]
    }

    /// Build a non-secret environment that lets OpenSSH ask the current Luma executable to
    /// retrieve one exact SSH password from Keychain.
    fn ssh_askpass_environment(
        &self,
        _account: &str,
    ) -> Result<Vec<(String, String)>, SshConfigError> {
        Err(SshConfigError::msg(
            "saved SSH passwords are unavailable on this platform",
        ))
    }
}
