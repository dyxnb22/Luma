use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SettingsError {
    #[error("settings io: {0}")]
    Io(String),
    #[error("version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },
    #[error("corrupt settings: {0}")]
    Corrupt(String),
    #[error("settings unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub schema_version: u32,
    pub settings_version: u64,
    pub enabled_modules: BTreeMap<String, bool>,
    pub notes_root: Option<String>,
    pub projects_roots: Vec<String>,
    #[serde(default)]
    pub notes_exclude_patterns: Vec<String>,
    pub clipboard_retention_days: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut enabled_modules = BTreeMap::new();
        enabled_modules.insert("luma.apps".into(), true);
        enabled_modules.insert("luma.clipboard".into(), true);
        enabled_modules.insert("luma.notes".into(), true);
        enabled_modules.insert("luma.fake".into(), false);
        Self {
            schema_version: 1,
            settings_version: 1,
            enabled_modules,
            notes_root: None,
            projects_roots: Vec::new(),
            notes_exclude_patterns: Vec::new(),
            clipboard_retention_days: 30,
        }
    }
}

/// Persistent application settings (CAS updates).
pub trait SettingsRepository: Send + Sync {
    fn load_or_default(&self) -> Result<AppSettings, SettingsError>;
    fn update_cas(
        &self,
        expected_version: u64,
        patch: AppSettings,
    ) -> Result<AppSettings, SettingsError>;
}
