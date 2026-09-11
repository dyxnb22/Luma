use crate::ports::{AppSettings, SettingsError, SettingsRepository};
use luma_storage::{ConfigError, ConfigStore};
use std::sync::Arc;

pub struct TomlSettingsRepository {
    store: Arc<ConfigStore>,
}

impl TomlSettingsRepository {
    pub fn new(store: Arc<ConfigStore>) -> Self {
        Self { store }
    }
}

fn map_err(err: ConfigError) -> SettingsError {
    match err {
        ConfigError::VersionConflict { expected, found } => {
            SettingsError::VersionConflict { expected, found }
        }
        ConfigError::Corrupt(path) => SettingsError::Corrupt(path.display().to_string()),
        ConfigError::LockTimeout => SettingsError::Unavailable(
            "settings lock timeout — another Luma instance may be saving".into(),
        ),
        ConfigError::Mutation(message) => SettingsError::Unavailable(message),
        other => SettingsError::Io(other.to_string()),
    }
}

impl SettingsRepository for TomlSettingsRepository {
    fn load_or_default(&self) -> Result<AppSettings, SettingsError> {
        self.store.load_or_default().map_err(map_err)
    }

    fn update_cas(
        &self,
        expected_version: u64,
        patch: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        self.store
            .update_cas(expected_version, patch)
            .map_err(map_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_storage::LumaSettings;
    use tempfile::tempdir;

    #[test]
    fn app_settings_is_luma_settings() {
        let dir = tempdir().unwrap();
        let store = Arc::new(ConfigStore::with_path(dir.path().join("settings.toml")));
        let repo = TomlSettingsRepository::new(store);
        let loaded = repo.load_or_default().unwrap();
        assert_eq!(loaded, LumaSettings::default());
    }
}
