use async_trait::async_trait;
use luma_storage::ResumeEditor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OpenEditorError {
    #[error("editor unavailable: {0}")]
    Unavailable(String),
    #[error("open editor failed: {0}")]
    Failed(String),
}

#[async_trait]
pub trait OpenEditorPort: Send + Sync {
    /// Open `path` with the given editor application (`Default` → system default via `open`).
    async fn open(&self, editor: ResumeEditor, path: &Path) -> Result<(), OpenEditorError>;

    /// Open Terminal.app at `cwd` (`open -a Terminal <cwd>`).
    async fn open_terminal(&self, cwd: &Path) -> Result<(), OpenEditorError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FakeEditorCall {
    Open {
        editor: ResumeEditor,
        path: PathBuf,
    },
    Terminal {
        cwd: PathBuf,
    },
}

/// Controllable fake for tests — never touches the GUI.
#[derive(Default)]
pub struct FakeOpenEditor {
    pub calls: Arc<Mutex<Vec<FakeEditorCall>>>,
    pub fail_next: Arc<Mutex<Option<OpenEditorError>>>,
    pub open_count: AtomicUsize,
}

impl FakeOpenEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_failure(err: OpenEditorError) -> Self {
        let f = Self::new();
        *f.fail_next.lock().expect("lock") = Some(err);
        f
    }

    fn take_failure(&self) -> Option<OpenEditorError> {
        self.fail_next.lock().expect("lock").take()
    }
}

#[async_trait]
impl OpenEditorPort for FakeOpenEditor {
    async fn open(&self, editor: ResumeEditor, path: &Path) -> Result<(), OpenEditorError> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        self.calls.lock().expect("lock").push(FakeEditorCall::Open {
            editor,
            path: path.to_path_buf(),
        });
        if let Some(err) = self.take_failure() {
            return Err(err);
        }
        Ok(())
    }

    async fn open_terminal(&self, cwd: &Path) -> Result<(), OpenEditorError> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        self.calls
            .lock()
            .expect("lock")
            .push(FakeEditorCall::Terminal {
                cwd: cwd.to_path_buf(),
            });
        if let Some(err) = self.take_failure() {
            return Err(err);
        }
        Ok(())
    }
}
