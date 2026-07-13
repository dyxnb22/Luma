use async_trait::async_trait;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum PasteboardError {
    #[error("pasteboard unavailable: {0}")]
    Unavailable(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait Pasteboard: Send + Sync {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError>;
    async fn write_text(&self, text: &str) -> Result<(), PasteboardError>;
}

/// Uses macOS `pbpaste` / `pbcopy` (Terminal-process identity).
pub struct MacPasteboard;

#[async_trait]
impl Pasteboard for MacPasteboard {
    async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
        let out = Command::new("pbpaste")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !out.status.success() {
            return Err(PasteboardError::Unavailable(format!(
                "pbpaste exited {}",
                out.status
            )));
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await?;
        }
        let status = child.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(PasteboardError::Unavailable(format!(
                "pbcopy exited {status}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemPb {
        text: tokio::sync::Mutex<Option<String>>,
    }

    #[async_trait]
    impl Pasteboard for MemPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.text.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            *self.text.lock().await = Some(text.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn mem_roundtrip() {
        let pb = MemPb {
            text: tokio::sync::Mutex::new(None),
        };
        pb.write_text("hi").await.unwrap();
        assert_eq!(pb.read_text().await.unwrap().as_deref(), Some("hi"));
    }
}
