use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors for a bounded UTF-8 import file read.
///
/// Variants deliberately carry no path, operating-system error, or file content so adapters can
/// report safe failures to modules without disclosing user input.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BoundedUtf8FileReadError {
    #[error("file input is unavailable")]
    Unavailable,
    #[error("file input must be a regular non-symlink file")]
    InvalidFile,
    #[error("file input exceeds the configured size limit")]
    TooLarge,
    #[error("file input is not valid UTF-8")]
    InvalidUtf8,
}

/// Reads a bounded UTF-8 file for a module import.
///
/// Implementations must reject directories and a symlink used as the final path component. They
/// must enforce `max_bytes` while reading, not only from metadata, because a file can change after
/// it is checked.
#[async_trait]
pub trait BoundedUtf8FileReaderPort: Send + Sync {
    async fn read_utf8(
        &self,
        path: &Path,
        max_bytes: usize,
    ) -> Result<String, BoundedUtf8FileReadError>;
}

/// Controllable fake for module tests. It never accesses the filesystem.
#[derive(Default)]
pub struct FakeBoundedUtf8FileReader {
    pub calls: Arc<Mutex<Vec<(PathBuf, usize)>>>,
    responses: Mutex<VecDeque<Result<String, BoundedUtf8FileReadError>>>,
}

impl FakeBoundedUtf8FileReader {
    pub fn with_text(text: impl Into<String>) -> Self {
        let reader = Self::default();
        reader.push_response(Ok(text.into()));
        reader
    }

    pub fn with_error(error: BoundedUtf8FileReadError) -> Self {
        let reader = Self::default();
        reader.push_response(Err(error));
        reader
    }

    pub fn push_response(&self, response: Result<String, BoundedUtf8FileReadError>) {
        self.responses.lock().expect("lock").push_back(response);
    }
}

#[async_trait]
impl BoundedUtf8FileReaderPort for FakeBoundedUtf8FileReader {
    async fn read_utf8(
        &self,
        path: &Path,
        max_bytes: usize,
    ) -> Result<String, BoundedUtf8FileReadError> {
        self.calls
            .lock()
            .expect("lock")
            .push((path.to_path_buf(), max_bytes));
        self.responses
            .lock()
            .expect("lock")
            .pop_front()
            .unwrap_or(Err(BoundedUtf8FileReadError::Unavailable))
    }
}
