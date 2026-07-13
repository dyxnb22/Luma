use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessibilityError {
    #[error("accessibility not trusted")]
    NotTrusted,
    #[error("paste synthesis failed: {0}")]
    PasteFailed(String),
}

#[async_trait]
pub trait AccessibilityPort: Send + Sync {
    fn is_trusted(&self) -> bool;
    /// Paste whatever is currently on the pasteboard via Cmd+V.
    async fn paste_clipboard(&self) -> Result<(), AccessibilityError>;
}

/// Deterministic fake for module tests.
pub struct FakeAccessibility {
    pub trusted: bool,
    pub paste_ok: bool,
}

#[async_trait]
impl AccessibilityPort for FakeAccessibility {
    fn is_trusted(&self) -> bool {
        self.trusted
    }

    async fn paste_clipboard(&self) -> Result<(), AccessibilityError> {
        if !self.trusted {
            return Err(AccessibilityError::NotTrusted);
        }
        if self.paste_ok {
            Ok(())
        } else {
            Err(AccessibilityError::PasteFailed("fake deny".into()))
        }
    }
}
