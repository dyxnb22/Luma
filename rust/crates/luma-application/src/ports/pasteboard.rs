use async_trait::async_trait;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasteboardError {
    #[error("pasteboard unavailable: {0}")]
    Unavailable(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasteboardSnapshot {
    pub text: Option<String>,
    /// False when the producer marked the pasteboard as concealed, transient or otherwise
    /// unsuitable for durable history. Direct user-requested paste still uses `read_text`.
    pub capture_allowed: bool,
}

impl Default for PasteboardSnapshot {
    fn default() -> Self {
        Self {
            text: None,
            capture_allowed: true,
        }
    }
}

#[async_trait]
pub trait PasteboardPort: Send + Sync {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError>;
    async fn read_for_capture(&self) -> Result<PasteboardSnapshot, PasteboardError> {
        Ok(PasteboardSnapshot {
            text: self.read_text().await?,
            capture_allowed: true,
        })
    }
    async fn write_text(&self, text: &str) -> Result<(), PasteboardError>;
}

/// Test double that records writes and never touches the system pasteboard.
#[derive(Default)]
pub struct FakePasteboard {
    text: Mutex<Option<String>>,
}

impl FakePasteboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_text(&self) -> Option<String> {
        self.text.lock().unwrap().clone()
    }
}

#[async_trait]
impl PasteboardPort for FakePasteboard {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
        Ok(self.text.lock().unwrap().clone())
    }

    async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
        *self.text.lock().unwrap() = Some(text.to_string());
        Ok(())
    }
}
