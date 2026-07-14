use crate::ports::{AppSettings, SettingsError, SettingsRepository};
use luma_storage::{ConfigError, ConfigStore, LumaSettings};
use std::sync::Arc;

pub struct TomlSettingsRepository {
    store: Arc<ConfigStore>,
}

impl TomlSettingsRepository {
    pub fn new(store: Arc<ConfigStore>) -> Self {
        Self { store }
    }
}

fn to_app(settings: LumaSettings) -> AppSettings {
    AppSettings {
        schema_version: settings.schema_version,
        settings_version: settings.settings_version,
        enabled_modules: settings.enabled_modules,
        notes_root: settings.notes_root,
        projects_roots: settings.projects_roots,
        notes_exclude_patterns: settings.notes_exclude_patterns,
        clipboard_retention_days: settings.clipboard_retention_days,
    }
}

fn to_storage(settings: AppSettings) -> LumaSettings {
    LumaSettings {
        schema_version: settings.schema_version,
        settings_version: settings.settings_version,
        enabled_modules: settings.enabled_modules,
        notes_root: settings.notes_root,
        projects_roots: settings.projects_roots,
        notes_exclude_patterns: settings.notes_exclude_patterns,
        clipboard_retention_days: settings.clipboard_retention_days,
    }
}

fn map_err(err: ConfigError) -> SettingsError {
    match err {
        ConfigError::VersionConflict { expected, found } => {
            SettingsError::VersionConflict { expected, found }
        }
        ConfigError::Corrupt(path) => SettingsError::Corrupt(path.display().to_string()),
        other => SettingsError::Io(other.to_string()),
    }
}

impl SettingsRepository for TomlSettingsRepository {
    fn load_or_default(&self) -> Result<AppSettings, SettingsError> {
        self.store.load_or_default().map(to_app).map_err(map_err)
    }

    fn update_cas(
        &self,
        expected_version: u64,
        patch: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        self.store
            .update_cas(expected_version, to_storage(patch))
            .map(to_app)
            .map_err(map_err)
    }
}
