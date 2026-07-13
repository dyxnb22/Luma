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
