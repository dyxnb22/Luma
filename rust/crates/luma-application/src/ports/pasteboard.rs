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

#[async_trait]
pub trait PasteboardPort: Send + Sync {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError>;
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
