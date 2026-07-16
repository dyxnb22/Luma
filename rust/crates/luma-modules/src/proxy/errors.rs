use luma_application::{ProfileStoreError, ProxyCoreError, SystemProxyError};
use luma_domain::FailureKind;

pub(super) fn proxy_failure(error: ProxyCoreError) -> FailureKind {
    match error {
        ProxyCoreError::PermissionRequired(guidance) => FailureKind::PermissionRequired {
            capability: "mihomo_controller".into(),
            guidance,
        },
        // Controller paths can include provider, group, or node labels. Keep adapter details out
        // of ActionOutcome, which is rendered and may be retained by callers.
        ProxyCoreError::Timeout(_) => FailureKind::Timeout {
            operation: "Mihomo controller request".into(),
        },
        ProxyCoreError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProxyCoreError::NotFound(_) => FailureKind::NotFound {
            entity: "Proxy item".into(),
        },
        ProxyCoreError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        ProxyCoreError::NotConfigured(remediation) => FailureKind::NotConfigured { remediation },
        ProxyCoreError::Unavailable(_) => FailureKind::Unavailable {
            reason: "Mihomo controller is unavailable".into(),
            retryable: true,
        },
    }
}

pub(super) fn system_failure(error: SystemProxyError) -> FailureKind {
    match error {
        SystemProxyError::PermissionRequired(guidance) => FailureKind::PermissionRequired {
            capability: "system_proxy".into(),
            guidance,
        },
        SystemProxyError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        SystemProxyError::Conflict => FailureKind::Conflict {
            reason: "System proxy changed outside Luma; it was not overwritten".into(),
        },
        SystemProxyError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

pub(super) fn profile_failure(error: ProfileStoreError) -> FailureKind {
    match error {
        ProfileStoreError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProfileStoreError::NotFound(entity) => FailureKind::NotFound { entity },
        ProfileStoreError::Timeout => FailureKind::Timeout {
            operation: "profile operation".into(),
        },
        ProfileStoreError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        ProfileStoreError::Conflict(reason) => FailureKind::Conflict { reason },
        ProfileStoreError::NotConfigured(remediation) => FailureKind::NotConfigured { remediation },
        ProfileStoreError::Unsupported(reason) => FailureKind::Unavailable {
            reason,
            retryable: false,
        },
        ProfileStoreError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}
