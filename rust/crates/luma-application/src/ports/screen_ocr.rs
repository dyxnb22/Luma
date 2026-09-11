use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_OCR_TEXT_BYTES: usize = 256 * 1024;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScreenOcrError {
    #[error("screen OCR cancelled")]
    Cancelled,
    #[error("Screen Recording permission is required")]
    PermissionRequired,
    #[error("no text was recognized")]
    Empty,
    #[error("screen capture is unavailable: {0}")]
    CaptureUnavailable(String),
    #[error("text recognition is unavailable: {0}")]
    RecognitionUnavailable(String),
}

#[async_trait]
pub trait ScreenOcrPort: Send + Sync {
    async fn recognize_region(&self, cancel: CancellationToken) -> Result<String, ScreenOcrError>;
}

pub struct FakeScreenOcr {
    outcomes: Mutex<VecDeque<Result<String, ScreenOcrError>>>,
    pub calls: Mutex<usize>,
}

impl FakeScreenOcr {
    pub fn new(outcomes: impl IntoIterator<Item = Result<String, ScreenOcrError>>) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().collect()),
            calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl ScreenOcrPort for FakeScreenOcr {
    async fn recognize_region(&self, cancel: CancellationToken) -> Result<String, ScreenOcrError> {
        if cancel.is_cancelled() {
            return Err(ScreenOcrError::Cancelled);
        }
        *self.calls.lock().expect("OCR calls lock") += 1;
        self.outcomes
            .lock()
            .expect("OCR outcomes lock")
            .pop_front()
            .unwrap_or_else(|| {
                Err(ScreenOcrError::CaptureUnavailable(
                    "fixture exhausted".into(),
                ))
            })
    }
}
