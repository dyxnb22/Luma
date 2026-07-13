use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DiagnosticsError {
    #[error("diagnostics io: {0}")]
    Io(String),
    #[error("diagnostics unavailable: {0}")]
    Unavailable(String),
}

/// Writes redacted diagnostic exports to durable storage.
pub trait DiagnosticsSink: Send + Sync {
    fn write_export(&self, file_name: &str, body: &[u8]) -> Result<PathBuf, DiagnosticsError>;
}
