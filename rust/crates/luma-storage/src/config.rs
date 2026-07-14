use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },
    #[error("corrupt config quarantined to {0}")]
    Corrupt(PathBuf),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LumaSettings {
    pub schema_version: u32,
    pub settings_version: u64,
    pub enabled_modules: BTreeMap<String, bool>,
    pub notes_root: Option<String>,
    #[serde(default)]
    pub projects_roots: Vec<String>,
    /// Glob patterns relative to notes_root (e.g. `private/*`).
    #[serde(default)]
    pub notes_exclude_patterns: Vec<String>,
    pub clipboard_retention_days: u32,
}

impl Default for LumaSettings {
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

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn luma_next_default() -> Result<Self, ConfigError> {
        ensure_luma_next_dirs()?;
        Ok(Self {
            path: luma_next_support_dir()?.join("settings.toml"),
        })
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_or_default(&self) -> Result<LumaSettings, ConfigError> {
        let marker = self.corrupt_marker_path();
        if marker.exists() {
            return Err(ConfigError::Corrupt(marker));
        }
        if !self.path.exists() {
            let settings = LumaSettings::default();
            self.save(&settings)?;
            return Ok(settings);
        }
        let raw = fs::read_to_string(&self.path)?;
        match toml::from_str::<LumaSettings>(&raw) {
            Ok(s) => Ok(s),
            Err(_) => {
                let quarantine = self.path.with_extension(format!(
                    "corrupt-{}.toml",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ));
                fs::rename(&self.path, &quarantine)?;
                // Block silent re-init that would re-enable modules.
                fs::write(
                    &marker,
                    format!(
                        "quarantined={}\nremediation=inspect quarantined file then delete this marker and run `luma config set` or restore settings.toml\n",
                        quarantine.display()
                    ),
                )?;
                Err(ConfigError::Corrupt(quarantine))
            }
        }
    }

    fn corrupt_marker_path(&self) -> PathBuf {
        self.path.with_extension("toml.corrupt-marker")
    }

    /// Clear the corrupt marker after an operator restores settings (tests / recovery).
    pub fn clear_corrupt_marker(&self) -> Result<(), ConfigError> {
        let marker = self.corrupt_marker_path();
        if marker.exists() {
            fs::remove_file(marker)?;
        }
        Ok(())
    }

    pub fn save(&self, settings: &LumaSettings) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(settings)?;
        let tmp = self.path.with_extension("toml.tmp");
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn update_cas(
        &self,
        expected_version: u64,
        mut patch: LumaSettings,
    ) -> Result<LumaSettings, ConfigError> {
        let current = self.load_or_default()?;
        if current.settings_version != expected_version {
            return Err(ConfigError::VersionConflict {
                expected: expected_version,
                found: current.settings_version,
            });
        }
        patch.settings_version = current.settings_version + 1;
        patch.schema_version = current.schema_version;
        self.save(&patch)?;
        Ok(patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trip_and_cas() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::with_path(dir.path().join("settings.toml"));
        let s = store.load_or_default().unwrap();
        assert_eq!(s.settings_version, 1);
        let mut next = s.clone();
        next.notes_root = Some("/tmp/notes".into());
        let saved = store.update_cas(1, next).unwrap();
        assert_eq!(saved.settings_version, 2);
        assert!(store.update_cas(1, saved.clone()).is_err());
    }

    #[test]
    fn corrupt_is_quarantined() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        fs::write(&path, "not = toml [[[").unwrap();
        let store = ConfigStore::with_path(path);
        let err = store.load_or_default().unwrap_err();
        assert!(matches!(err, ConfigError::Corrupt(_)));
    }
}
