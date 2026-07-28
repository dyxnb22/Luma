//! Opens one explicit macOS Privacy & Security pane on a user action.

use async_trait::async_trait;
use luma_application::{SystemSettingsError, SystemSettingsPane, SystemSettingsPort};

pub struct MacSystemSettings;

fn pane_url(pane: SystemSettingsPane) -> &'static str {
    match pane {
        SystemSettingsPane::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        SystemSettingsPane::ScreenRecording => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
    }
}

#[async_trait]
impl SystemSettingsPort for MacSystemSettings {
    async fn open(&self, pane: SystemSettingsPane) -> Result<(), SystemSettingsError> {
        let status = tokio::process::Command::new("/usr/bin/open")
            .arg(pane_url(pane))
            .status()
            .await
            .map_err(|error| SystemSettingsError::OpenFailed {
                pane: pane.display_name().into(),
                reason: error.to_string(),
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(SystemSettingsError::OpenFailed {
                pane: pane.display_name().into(),
                reason: format!("open exited {status}"),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_panes_have_explicit_deep_links() {
        assert!(pane_url(SystemSettingsPane::Accessibility).contains("Privacy_Accessibility"));
        assert!(pane_url(SystemSettingsPane::ScreenRecording).contains("Privacy_ScreenCapture"));
    }
}
