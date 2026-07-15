//! Paste into a previously recorded target application (not the Luma terminal).

use crate::ports::{
    AccessibilityError, AccessibilityPort, PasteboardError, PasteboardPort, WindowCatalogPort,
    WindowError,
};
use luma_domain::FailureKind;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub const AX_PASTE_TIMEOUT: Duration = Duration::from_secs(3);

fn map_window_err(err: WindowError) -> FailureKind {
    match err {
        WindowError::PermissionRequired {
            capability,
            guidance,
        } => FailureKind::PermissionRequired {
            capability,
            guidance,
        },
        WindowError::NotFound(entity) => FailureKind::NotFound { entity },
        WindowError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

/// Write `text` to the pasteboard, refocus the recorded target app, verify frontmost, then Cmd-V.
pub async fn paste_to_target_app(
    catalog: Arc<dyn WindowCatalogPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    accessibility: Arc<dyn AccessibilityPort>,
    text: &str,
) -> Result<(), FailureKind> {
    if !accessibility.is_trusted() {
        return Err(FailureKind::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Grant Accessibility to paste into other apps".into(),
        });
    }
    let Some(target) = catalog.paste_target_app().await else {
        return Err(FailureKind::Unavailable {
            reason: "no paste target — focus another app before opening Luma".into(),
            retryable: false,
        });
    };
    pasteboard
        .write_text(text)
        .await
        .map_err(|e: PasteboardError| FailureKind::Unavailable {
            reason: e.to_string(),
            retryable: true,
        })?;
    timeout(AX_PASTE_TIMEOUT, catalog.focus_app_by_name(&target))
        .await
        .map_err(|_| FailureKind::Timeout {
            operation: "focus paste target".into(),
        })?
        .map_err(map_window_err)?;
    let front = timeout(AX_PASTE_TIMEOUT, catalog.frontmost_app_name())
        .await
        .map_err(|_| FailureKind::Timeout {
            operation: "verify paste target".into(),
        })?
        .map_err(map_window_err)?;
    if front.as_deref() != Some(target.as_str()) {
        return Err(FailureKind::Unavailable {
            reason: format!("could not focus {target} for paste"),
            retryable: true,
        });
    }
    timeout(AX_PASTE_TIMEOUT, accessibility.paste_clipboard())
        .await
        .map_err(|_| FailureKind::Timeout {
            operation: "accessibility paste".into(),
        })?
        .map_err(|e: AccessibilityError| match e {
            AccessibilityError::NotTrusted => FailureKind::PermissionRequired {
                capability: "accessibility".into(),
                guidance: "Grant Accessibility to paste into other apps".into(),
            },
            AccessibilityError::PasteFailed(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{FakeAccessibility, FakePasteboard, FakeWindowCatalog, WindowEntry};
    use std::sync::Arc;

    fn safari_catalog() -> Arc<FakeWindowCatalog> {
        Arc::new(FakeWindowCatalog::with_entries(
            vec![WindowEntry {
                id: "pid:1|num:1".into(),
                app_name: "Safari".into(),
                app_bundle_id: None,
                title: "Page".into(),
                is_on_screen: true,
                layer: 0,
                owner_pid: 1,
            }],
            Some("Safari".into()),
        ))
    }

    #[tokio::test]
    async fn paste_success_when_target_focused() {
        let catalog = safari_catalog();
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility {
            trusted: true,
            paste_ok: true,
        });
        paste_to_target_app(catalog.clone(), pasteboard, accessibility, "hello")
            .await
            .expect("paste ok");
        let calls = catalog.focus_app_calls.lock().await;
        assert_eq!(calls.as_slice(), &["Safari".to_string()]);
    }

    #[tokio::test]
    async fn paste_unavailable_without_target() {
        let catalog = Arc::new(FakeWindowCatalog::default());
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility {
            trusted: true,
            paste_ok: true,
        });
        let err = paste_to_target_app(catalog, pasteboard, accessibility, "x")
            .await
            .unwrap_err();
        assert!(matches!(err, FailureKind::Unavailable { .. }));
    }

    #[tokio::test]
    async fn paste_not_trusted_is_permission_required() {
        let catalog = safari_catalog();
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility {
            trusted: false,
            paste_ok: true,
        });
        let err = paste_to_target_app(catalog, pasteboard, accessibility, "x")
            .await
            .unwrap_err();
        assert!(matches!(err, FailureKind::PermissionRequired { .. }));
    }
}
