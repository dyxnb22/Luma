use luma_application::WindowEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordbookStatus {
    NotConfigured,
    Unavailable(String),
    Ready {
        due: i64,
        reviewed_today: i64,
        goal: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowsStatus {
    Unavailable(String),
    Ready(Vec<WindowEntry>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuSnapshot {
    pub wordbook: WordbookStatus,
    pub windows: WindowsStatus,
    pub cli_available: bool,
    pub launch_at_login: bool,
    pub global_warning: Option<String>,
}

impl Default for MenuSnapshot {
    fn default() -> Self {
        Self {
            wordbook: WordbookStatus::NotConfigured,
            windows: WindowsStatus::Ready(Vec::new()),
            cli_available: false,
            launch_at_login: false,
            global_warning: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Refresh,
    OpenLuma,
    OpenSettings,
    ReviewDue,
    FocusWindow(usize),
    ToggleLaunchAtLogin,
    Quit,
}
