//! Port for opening paths in the user's preferred viewer.
//! Production may shell out to `/usr/bin/open`; tests must use [`FakeOpenPath`].

use async_trait::async_trait;
use std::path::Path;

pub use luma_application::{FakeOpenPath, OpenPathError, OpenPathPort as OpenPath};

/// macOS `/usr/bin/open` adapter. Do **not** use in automated tests.
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
