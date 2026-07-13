use async_trait::async_trait;
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
