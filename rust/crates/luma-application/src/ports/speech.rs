use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpeechError {
    #[error("speech failed: {0}")]
    Failed(String),
    #[error("speech unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeechAccent {
    Uk,
    Default,
}

#[async_trait]
pub trait SpeechPort: Send + Sync {
    async fn speak(&self, text: &str, accent: SpeechAccent) -> Result<(), SpeechError>;
}

/// Controllable fake for tests — never shells out to `say`.
#[derive(Default)]
pub struct FakeSpeech {
    pub calls: Arc<Mutex<Vec<(String, SpeechAccent)>>>,
    pub fail_next: Arc<Mutex<bool>>,
    pub speak_count: AtomicUsize,
}

impl FakeSpeech {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SpeechPort for FakeSpeech {
    async fn speak(&self, text: &str, accent: SpeechAccent) -> Result<(), SpeechError> {
        self.speak_count.fetch_add(1, Ordering::SeqCst);
        self.calls
            .lock()
            .expect("lock")
            .push((text.to_string(), accent));
        let mut fail = self.fail_next.lock().expect("lock");
        if *fail {
            *fail = false;
            return Err(SpeechError::Failed("fake speech denied".into()));
        }
        Ok(())
    }
}
