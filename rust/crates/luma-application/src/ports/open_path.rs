use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenPathError {
    #[error("open failed: {0}")]
    Failed(String),
    #[error("open unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait OpenPathPort: Send + Sync {
    async fn open(&self, path: &Path) -> Result<(), OpenPathError>;
}

/// Controllable fake for tests — never touches the GUI.
#[derive(Default)]
pub struct FakeOpenPath {
    pub calls: Arc<Mutex<Vec<std::path::PathBuf>>>,
    pub fail_next: Arc<Mutex<bool>>,
    pub open_count: AtomicUsize,
}

impl FakeOpenPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_failure() -> Self {
        let f = Self::new();
        *f.fail_next.lock().expect("lock") = true;
        f
    }
}

#[async_trait]
impl OpenPathPort for FakeOpenPath {
    async fn open(&self, path: &Path) -> Result<(), OpenPathError> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        self.calls.lock().expect("lock").push(path.to_path_buf());
        let mut fail = self.fail_next.lock().expect("lock");
        if *fail {
            *fail = false;
            return Err(OpenPathError::Failed("fake open denied".into()));
        }
        Ok(())
    }
}
