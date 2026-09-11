use async_trait::async_trait;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessibilityError {
    #[error("accessibility not trusted")]
    NotTrusted,
    #[error("paste synthesis failed: {0}")]
    PasteFailed(String),
}

/// True when frontmost still matches the paste target (gate-side re-check).
pub fn frontmost_matches_paste_target(frontmost: Option<&str>, expected_app: &str) -> bool {
    frontmost == Some(expected_app)
}

#[async_trait]
pub trait AccessibilityPort: Send + Sync {
    fn is_trusted(&self) -> bool;
    /// Paste via Cmd+V only if `expected_app` is still frontmost when synthesis runs.
    async fn paste_clipboard(&self, expected_app: &str) -> Result<(), AccessibilityError>;
    /// Invalidate pending AX work and wait for an already-started side effect to finish.
    fn abandon_pending_ax_ops(&self) {}
}

/// Deterministic fake for module tests.
pub struct FakeAccessibility {
    pub trusted: bool,
    pub paste_ok: bool,
    /// When `Some`, simulates frontmost at synthesize time (must match `expected_app`).
    pub frontmost_at_gate: Mutex<Option<String>>,
}

impl FakeAccessibility {
    pub fn new(trusted: bool, paste_ok: bool) -> Self {
        Self {
            trusted,
            paste_ok,
            frontmost_at_gate: Mutex::new(None),
        }
    }
}

#[async_trait]
impl AccessibilityPort for FakeAccessibility {
    fn is_trusted(&self) -> bool {
        self.trusted
    }

    async fn paste_clipboard(&self, expected_app: &str) -> Result<(), AccessibilityError> {
        if !self.trusted {
            return Err(AccessibilityError::NotTrusted);
        }
        let front = self.frontmost_at_gate.lock().unwrap().clone();
        if let Some(ref front) = front {
            if !frontmost_matches_paste_target(Some(front.as_str()), expected_app) {
                return Err(AccessibilityError::PasteFailed(format!(
                    "frontmost changed before paste (expected {expected_app}, got {front})"
                )));
            }
        }
        if self.paste_ok {
            Ok(())
        } else {
            Err(AccessibilityError::PasteFailed("fake deny".into()))
        }
    }
}
