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

async fn abandon_ax(
    catalog: Arc<dyn WindowCatalogPort>,
    accessibility: Arc<dyn AccessibilityPort>,
) {
    // The platform adapters may need to wait for an already-started AX call to
    // leave its side-effect gate. Keep that synchronous wait off the runtime.
    let _ = tokio::task::spawn_blocking(move || {
        catalog.abandon_pending_ax_ops();
        accessibility.abandon_pending_ax_ops();
    })
    .await;
}

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

pub const NO_PASTE_TARGET_REASON: &str =
    "no paste target — focus the destination with Hub or `win`, then paste again";

/// Resolve paste destination from cache, then the launch-time previous-frontmost label.
///
/// Do **not** re-enumerate live windows here: once Luma is focused, a live snapshot /
/// frontmost probe often points at an unrelated app and would paste into the wrong place.
async fn resolve_paste_target(catalog: &dyn WindowCatalogPort) -> Option<String> {
    if let Some(target) = catalog.paste_target_app().await {
        return Some(target);
    }
    if let Some(prev) = catalog.previous_frontmost_app().await {
        catalog.set_paste_target_app(Some(prev.clone())).await;
        return Some(prev);
    }
    None
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
    let Some(target) = resolve_paste_target(catalog.as_ref()).await else {
        return Err(FailureKind::Unavailable {
            reason: NO_PASTE_TARGET_REASON.into(),
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
    match timeout(AX_PASTE_TIMEOUT, catalog.focus_app_by_name(&target)).await {
        Err(_) => {
            abandon_ax(catalog.clone(), accessibility.clone()).await;
            return Err(FailureKind::Timeout {
                operation: "focus paste target".into(),
            });
        }
        Ok(result) => result.map_err(map_window_err)?,
    }
    let front = match timeout(AX_PASTE_TIMEOUT, catalog.frontmost_app_name()).await {
        Err(_) => {
            abandon_ax(catalog.clone(), accessibility.clone()).await;
            return Err(FailureKind::Timeout {
                operation: "verify paste target".into(),
            });
        }
        Ok(result) => result.map_err(map_window_err)?,
    };
    if front.as_deref() != Some(target.as_str()) {
        abandon_ax(catalog.clone(), accessibility.clone()).await;
        return Err(FailureKind::Unavailable {
            reason: format!("could not focus {target} for paste"),
            retryable: true,
        });
    }
    match timeout(AX_PASTE_TIMEOUT, accessibility.paste_clipboard(&target)).await {
        Err(_) => {
            abandon_ax(catalog, accessibility).await;
            Err(FailureKind::Timeout {
                operation: "accessibility paste".into(),
            })
        }
        Ok(result) => result.map_err(|e: AccessibilityError| match e {
            AccessibilityError::NotTrusted => FailureKind::PermissionRequired {
                capability: "accessibility".into(),
                guidance: "Grant Accessibility to paste into other apps".into(),
            },
            AccessibilityError::PasteFailed(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
        }),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{
        frontmost_matches_paste_target, FakeAccessibility, FakePasteboard, FakeWindowCatalog,
        WindowEntry,
    };
    use std::sync::{Arc, Mutex};

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
        let accessibility = Arc::new(FakeAccessibility::new(true, true));
        paste_to_target_app(catalog.clone(), pasteboard, accessibility, "hello")
            .await
            .expect("paste ok");
        let calls = catalog.focus_app_calls.lock().await;
        assert_eq!(calls.as_slice(), &["Safari".to_string()]);
    }

    #[tokio::test]
    async fn paste_aborts_if_frontmost_changes_before_synthesis() {
        assert!(frontmost_matches_paste_target(Some("Safari"), "Safari"));
        assert!(!frontmost_matches_paste_target(Some("Mail"), "Safari"));
        let catalog = safari_catalog();
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility {
            trusted: true,
            paste_ok: true,
            frontmost_at_gate: Mutex::new(Some("Mail".into())),
        });
        let err = paste_to_target_app(catalog, pasteboard, accessibility, "hello")
            .await
            .unwrap_err();
        assert!(
            matches!(err, FailureKind::Unavailable { ref reason, .. } if reason.contains("frontmost")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn paste_unavailable_without_target() {
        let catalog = Arc::new(FakeWindowCatalog::default());
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility::new(true, true));
        let err = paste_to_target_app(catalog.clone(), pasteboard, accessibility, "x")
            .await
            .unwrap_err();
        match err {
            FailureKind::Unavailable { reason, retryable } => {
                assert_eq!(reason, NO_PASTE_TARGET_REASON);
                assert!(!retryable);
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
        assert_eq!(
            *catalog.snapshot_calls.lock().await,
            0,
            "must not re-snapshot live windows when resolving paste target"
        );
    }

    #[tokio::test]
    async fn paste_uses_previous_frontmost_when_paste_target_cleared() {
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
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
        ));
        // Clear cached paste target so paste must recover from previous_frontmost.
        catalog.set_paste_target_app(None).await;
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility::new(true, true));
        paste_to_target_app(catalog.clone(), pasteboard, accessibility, "hello")
            .await
            .expect("paste from previous_frontmost");
        assert_eq!(*catalog.snapshot_calls.lock().await, 0);
        assert_eq!(catalog.paste_target_app().await.as_deref(), Some("Safari"));
    }

    #[tokio::test]
    async fn paste_does_not_use_live_frontmost_as_fallback() {
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
            vec![WindowEntry {
                id: "pid:2|num:1".into(),
                app_name: "Mail".into(),
                app_bundle_id: None,
                title: "Inbox".into(),
                is_on_screen: true,
                layer: 0,
                owner_pid: 2,
            }],
            None,
        ));
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility::new(true, true));
        let err = paste_to_target_app(catalog.clone(), pasteboard, accessibility, "x")
            .await
            .unwrap_err();
        assert!(matches!(err, FailureKind::Unavailable { .. }));
        assert!(catalog.focus_app_calls.lock().await.is_empty());
    }

    #[tokio::test]
    async fn paste_not_trusted_is_permission_required() {
        let catalog = safari_catalog();
        let pasteboard = Arc::new(FakePasteboard::new());
        let accessibility = Arc::new(FakeAccessibility::new(false, true));
        let err = paste_to_target_app(catalog, pasteboard, accessibility, "x")
            .await
            .unwrap_err();
        assert!(matches!(err, FailureKind::PermissionRequired { .. }));
    }
}
