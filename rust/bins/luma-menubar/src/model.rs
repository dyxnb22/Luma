use luma_application::WindowEntry;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordbookStatus {
    NotConfigured,
    Unavailable(String),
    Ready {
        due: i64,
        reviewed_today: i64,
        goal: i64,
    },
    Stale {
        due: i64,
        reviewed_today: i64,
        goal: i64,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowsStatus {
    PermissionRequired(String),
    Unavailable(String),
    Ready(Vec<WindowEntry>),
    Stale {
        windows: Vec<WindowEntry>,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliStatus {
    Available,
    Unavailable(String),
}

impl CliStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoginItemState {
    NotRegistered,
    Enabled,
    RequiresApproval,
    NotFound,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionStatus {
    Succeeded(String),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuSnapshot {
    pub generation: u64,
    pub captured_at_unix: u64,
    pub wordbook: WordbookStatus,
    pub windows: WindowsStatus,
    pub cli: CliStatus,
    pub login_item: LoginItemState,
    pub global_warning: Option<String>,
    pub last_action: Option<ActionStatus>,
}

pub type SharedMenuSnapshot = Arc<Mutex<MenuSnapshot>>;

impl Default for MenuSnapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            captured_at_unix: 0,
            wordbook: WordbookStatus::NotConfigured,
            windows: WindowsStatus::Ready(Vec::new()),
            cli: CliStatus::Unavailable("Luma CLI not found".into()),
            login_item: LoginItemState::NotRegistered,
            global_warning: None,
            last_action: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Refresh,
    OpenLuma,
    OpenSettings,
    ReviewDue,
    FocusWindow(String),
    ToggleLaunchAtLogin,
    Quit,
}
