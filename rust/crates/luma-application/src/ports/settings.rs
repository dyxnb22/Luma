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

/// Persistent settings schema (single source of truth: `luma_storage::LumaSettings`).
pub type AppSettings = luma_storage::LumaSettings;

/// Persistent application settings (CAS updates).
pub trait SettingsRepository: Send + Sync {
    fn load_or_default(&self) -> Result<AppSettings, SettingsError>;
    fn update_cas(
        &self,
        expected_version: u64,
        patch: AppSettings,
    ) -> Result<AppSettings, SettingsError>;
}
