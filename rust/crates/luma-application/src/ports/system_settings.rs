use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSettingsPane {
    Accessibility,
    ScreenRecording,
}

impl SystemSettingsPane {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Accessibility => "Accessibility",
            Self::ScreenRecording => "Screen & System Audio Recording",
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SystemSettingsError {
    #[error("could not open {pane}: {reason}")]
    OpenFailed { pane: String, reason: String },
}

#[async_trait]
pub trait SystemSettingsPort: Send + Sync {
    async fn open(&self, pane: SystemSettingsPane) -> Result<(), SystemSettingsError>;
}

/// Deterministic module-test adapter. It never opens System Settings.
#[derive(Default)]
pub struct FakeSystemSettings {
    pub calls: Arc<Mutex<Vec<SystemSettingsPane>>>,
    pub fail_next: Arc<Mutex<bool>>,
}

#[async_trait]
impl SystemSettingsPort for FakeSystemSettings {
    async fn open(&self, pane: SystemSettingsPane) -> Result<(), SystemSettingsError> {
        self.calls.lock().expect("system settings calls").push(pane);
        let mut fail = self.fail_next.lock().expect("system settings failure");
        if *fail {
            *fail = false;
            return Err(SystemSettingsError::OpenFailed {
                pane: pane.display_name().into(),
                reason: "fake open denied".into(),
            });
        }
        Ok(())
    }
}
