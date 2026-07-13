//! Port for opening paths in the user's preferred viewer.
//! Production may shell out to `/usr/bin/open`; tests and soak must use [`FakeOpenPath`].

use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum OpenPathError {
    #[error("open failed: {0}")]
    Failed(String),
    #[error("open unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait OpenPath: Send + Sync {
    async fn open(&self, path: &Path) -> Result<(), OpenPathError>;
}

/// macOS `/usr/bin/open` adapter. Do **not** use in automated tests or soak.
pub struct MacOpenPath;

#[async_trait]
impl OpenPath for MacOpenPath {
    async fn open(&self, path: &Path) -> Result<(), OpenPathError> {
        let status = tokio::process::Command::new("/usr/bin/open")
            .arg(path)
            .status()
            .await
            .map_err(|e| OpenPathError::Failed(e.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(OpenPathError::Failed(format!("open exited {status}")))
        }
    }
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
impl OpenPath for FakeOpenPath {
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
