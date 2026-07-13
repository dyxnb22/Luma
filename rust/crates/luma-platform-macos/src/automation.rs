//! Apple Events / Automation capability probe (Browser Tabs).

use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomationAuth {
    NotDetermined,
    Denied,
    Authorized,
}

#[derive(Debug, Error)]
pub enum AutomationError {
    #[error("automation not authorized")]
    NotAuthorized,
    #[error("automation unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[async_trait]
pub trait Automation: Send + Sync {
    async fn auth_status(&self) -> AutomationAuth;
    async fn list_tabs_cached(&self) -> Result<Vec<BrowserTab>, AutomationError>;
}

/// Bare CLI cannot safely trigger Automation prompts; modules show PermissionRequired.
pub struct MacAutomation;

#[async_trait]
impl Automation for MacAutomation {
    async fn auth_status(&self) -> AutomationAuth {
        AutomationAuth::NotDetermined
    }

    async fn list_tabs_cached(&self) -> Result<Vec<BrowserTab>, AutomationError> {
        Err(AutomationError::Unavailable(
            "Browser Automation requires signed identity + user consent; cache empty until spike host".into(),
        ))
    }
}

pub struct FakeAutomation {
    pub status: AutomationAuth,
    pub tabs: Vec<BrowserTab>,
}

#[async_trait]
impl Automation for FakeAutomation {
    async fn auth_status(&self) -> AutomationAuth {
        self.status
    }

    async fn list_tabs_cached(&self) -> Result<Vec<BrowserTab>, AutomationError> {
        if self.status != AutomationAuth::Authorized {
            return Err(AutomationError::NotAuthorized);
        }
        Ok(self.tabs.clone())
    }
}
