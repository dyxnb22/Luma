use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured failure taxonomy. Permission / warming / not-configured / timeout
/// must never be collapsed into a silent empty result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FailureKind {
    PermissionRequired {
        capability: String,
        guidance: String,
    },
    NotConfigured {
        remediation: String,
    },
    Warming {
        progress: Option<String>,
    },
    Unavailable {
        reason: String,
        retryable: bool,
    },
    Timeout {
        operation: String,
    },
    Cancelled,
    InvalidInput {
        field: String,
        message: String,
    },
    NotFound {
        entity: String,
    },
    Conflict {
        reason: String,
    },
    SecurityDenied {
        reason: String,
    },
    Io {
        context: String,
    },
    Internal {
        correlation_id: String,
    },
}

impl FailureKind {
    pub fn is_error(&self) -> bool {
        !matches!(self, FailureKind::Cancelled | FailureKind::Warming { .. })
    }

    /// Stable human-readable summary for status lines / logs.
    pub fn display_message(&self) -> String {
        match self {
            FailureKind::PermissionRequired {
                capability,
                guidance,
            } => format!("permission_required ({capability}): {guidance}"),
            FailureKind::NotConfigured { remediation } => {
                format!("not_configured: {remediation}")
            }
            FailureKind::Warming { progress } => match progress {
                Some(p) => format!("warming: {p}"),
                None => "warming".into(),
            },
            FailureKind::Unavailable { reason, retryable } => {
                format!("unavailable (retryable={retryable}): {reason}")
            }
            FailureKind::Timeout { operation } => format!("timeout: {operation}"),
            FailureKind::Cancelled => "cancelled".into(),
            FailureKind::InvalidInput { field, message } => {
                format!("invalid_input ({field}): {message}")
            }
            FailureKind::NotFound { entity } => format!("not_found: {entity}"),
            FailureKind::Conflict { reason } => format!("conflict: {reason}"),
            FailureKind::SecurityDenied { reason } => format!("security_denied: {reason}"),
            FailureKind::Io { context } => format!("io: {context}"),
            FailureKind::Internal { correlation_id } => {
                format!("internal [{correlation_id}]")
            }
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}")]
pub struct DomainError {
    pub kind: FailureKind,
}

impl DomainError {
    pub fn new(kind: FailureKind) -> Self {
        Self { kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_is_not_error() {
        assert!(!FailureKind::Cancelled.is_error());
    }

    #[test]
    fn permission_is_error() {
        let kind = FailureKind::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Enable AX".into(),
        };
        assert!(kind.is_error());
    }
}
