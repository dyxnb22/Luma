//! Open paths in a specific editor app via `/usr/bin/open -a`.
//! Tests must use [`FakeOpenEditor`].

use async_trait::async_trait;
use luma_application::{OpenEditorError, OpenEditorPort};
use luma_storage::ResumeEditor;
use std::path::Path;
use tokio::process::Command;

/// macOS adapter. Do **not** use in automated tests.
pub struct MacOpenEditor;

impl MacOpenEditor {
    async fn open_with_app(app: &str, path: &Path) -> Result<(), OpenEditorError> {
        let output = Command::new("/usr/bin/open")
            .args(["-a", app])
            .arg(path)
            .output()
            .await
            .map_err(|e| OpenEditorError::Failed(e.to_string()))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let lower = stderr.to_ascii_lowercase();
        if lower.contains("unable to find application")
            || lower.contains("couldn't find")
            || lower.contains("application not found")
        {
            return Err(OpenEditorError::Unavailable(format!(
                "{app} is not installed"
            )));
        }
        Err(OpenEditorError::Failed(if stderr.is_empty() {
            format!("open -a {app} exited {}", output.status)
        } else {
            stderr
        }))
    }
}

#[async_trait]
impl OpenEditorPort for MacOpenEditor {
    async fn open(&self, editor: ResumeEditor, path: &Path) -> Result<(), OpenEditorError> {
        match editor.app_name() {
            Some(app) => Self::open_with_app(app, path).await,
            None => {
                let status = Command::new("/usr/bin/open")
                    .arg(path)
                    .status()
                    .await
                    .map_err(|e| OpenEditorError::Failed(e.to_string()))?;
                if status.success() {
                    Ok(())
                } else {
                    Err(OpenEditorError::Failed(format!("open exited {status}")))
                }
            }
        }
    }

    async fn open_terminal(&self, cwd: &Path) -> Result<(), OpenEditorError> {
        Self::open_with_app("Terminal", cwd).await
    }
}
