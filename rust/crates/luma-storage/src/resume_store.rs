//! Resume contexts: explicit work-entry snapshots under LumaNext (JSON).

use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const RESUME_SCHEMA_VERSION: u32 = 1;
pub const RESUME_STORE_FILENAME: &str = "resume-contexts.json";

#[derive(Debug, Error)]
pub enum ResumeStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("corrupt resume store at {path}: {message}")]
    Corrupt { path: PathBuf, message: String },
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeEditor {
    Cursor,
    VsCode,
    IntelliJ,
    Default,
}

impl ResumeEditor {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cursor" => Some(Self::Cursor),
            "vscode" | "vs code" | "code" | "visual studio code" => Some(Self::VsCode),
            "intellij" | "idea" | "intellij idea" => Some(Self::IntelliJ),
            "default" | "system" | "finder" => Some(Self::Default),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::VsCode => "vscode",
            Self::IntelliJ => "intellij",
            Self::Default => "default",
        }
    }

    /// macOS `open -a` application name. `Default` has none (use OpenPath).
    pub fn app_name(&self) -> Option<&'static str> {
        match self {
            Self::Cursor => Some("Cursor"),
            Self::VsCode => Some("Visual Studio Code"),
            Self::IntelliJ => Some("IntelliJ IDEA"),
            Self::Default => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeRecipeRef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeContext {
    pub name: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipes: Vec<ResumeRecipeRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<ResumeEditor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_project_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_cwd: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_resumed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ResumeStoreFile {
    schema_version: u32,
    #[serde(default)]
    contexts: Vec<ResumeContext>,
}

pub struct ResumeStore {
    path: PathBuf,
}

impl ResumeStore {
    pub fn luma_next_default() -> Result<Self, ResumeStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join(RESUME_STORE_FILENAME))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, ResumeStoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list(&self) -> Result<Vec<ResumeContext>, ResumeStoreError> {
        let mut contexts = self.load()?.contexts;
        sort_contexts_by_recency(&mut contexts);
        Ok(contexts)
    }

    pub fn get(&self, name: &str) -> Result<Option<ResumeContext>, ResumeStoreError> {
        let key = normalize_name(name)?;
        Ok(self
            .load()?
            .contexts
            .into_iter()
            .find(|c| c.name == key))
    }

    /// Insert or replace by name. Paths in the context should already be normalized.
    pub fn upsert(&self, context: ResumeContext) -> Result<ResumeContext, ResumeStoreError> {
        let key = normalize_name(&context.name)?;
        let mut file = self.load()?;
        let now = now_iso();
        let mut saved = context;
        saved.name = key.clone();
        if saved.display_name.trim().is_empty() {
            saved.display_name = key.clone();
        }
        if let Some(existing) = file.contexts.iter_mut().find(|c| c.name == key) {
            saved.created_at = existing.created_at.clone();
            if saved.updated_at.is_empty() {
                saved.updated_at = now;
            }
            *existing = saved.clone();
        } else {
            if saved.created_at.is_empty() {
                saved.created_at = now.clone();
            }
            if saved.updated_at.is_empty() {
                saved.updated_at = now;
            }
            file.contexts.push(saved.clone());
        }
        self.save(&file)?;
        Ok(saved)
    }

    pub fn delete(&self, name: &str) -> Result<(), ResumeStoreError> {
        let key = normalize_name(name)?;
        let mut file = self.load()?;
        let before = file.contexts.len();
        file.contexts.retain(|c| c.name != key);
        if file.contexts.len() == before {
            return Err(ResumeStoreError::NotFound(key));
        }
        self.save(&file)
    }

    pub fn mark_resumed(&self, name: &str) -> Result<ResumeContext, ResumeStoreError> {
        let key = normalize_name(name)?;
        let mut file = self.load()?;
        let now = now_iso();
        let Some(ctx) = file.contexts.iter_mut().find(|c| c.name == key) else {
            return Err(ResumeStoreError::NotFound(key));
        };
        ctx.last_resumed_at = Some(now.clone());
        ctx.updated_at = now;
        let out = ctx.clone();
        self.save(&file)?;
        Ok(out)
    }

    /// Replace a corrupt/missing store with an empty file. Does not delete a corrupt
    /// original until a corrupt backup is written (best-effort).
    pub fn rebuild_empty(&self) -> Result<(), ResumeStoreError> {
        if self.path.exists() {
            let _ = self.backup_corrupt("manual rebuild");
        }
        self.save(&ResumeStoreFile {
            schema_version: RESUME_SCHEMA_VERSION,
            contexts: Vec::new(),
        })
    }

    fn load(&self) -> Result<ResumeStoreFile, ResumeStoreError> {
        if !self.path.exists() {
            return Ok(ResumeStoreFile {
                schema_version: RESUME_SCHEMA_VERSION,
                contexts: Vec::new(),
            });
        }
        let raw = fs::read_to_string(&self.path)?;
        if raw.trim().is_empty() {
            return Ok(ResumeStoreFile {
                schema_version: RESUME_SCHEMA_VERSION,
                contexts: Vec::new(),
            });
        }
        match serde_json::from_str::<ResumeStoreFile>(&raw) {
            Ok(file) => Ok(file),
            Err(err) => {
                let _ = self.backup_corrupt(&err.to_string());
                Err(ResumeStoreError::Corrupt {
                    path: self.path.clone(),
                    message: err.to_string(),
                })
            }
        }
    }

    fn save(&self, file: &ResumeStoreFile) -> Result<(), ResumeStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(file)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn backup_corrupt(&self, message: &str) -> Result<PathBuf, ResumeStoreError> {
        let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup = self
            .path
            .with_file_name(format!("{RESUME_STORE_FILENAME}.corrupt.{stamp}"));
        if self.path.exists() {
            fs::copy(&self.path, &backup)?;
        } else {
            fs::write(&backup, format!("# corrupt backup note: {message}\n"))?;
        }
        Ok(backup)
    }
}

pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn normalize_name(name: &str) -> Result<String, ResumeStoreError> {
    let key = name.trim().to_ascii_lowercase();
    if key.is_empty() {
        return Err(ResumeStoreError::InvalidName("empty".into()));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ResumeStoreError::InvalidName(format!(
            "use letters, digits, '-' or '_': {name}"
        )));
    }
    Ok(key)
}

/// Persist absolute paths only. Existing paths are canonicalized when possible.
pub fn normalize_path_for_store(input: &str) -> Result<String, ResumeStoreError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ResumeStoreError::InvalidPath("empty path".into()));
    }
    let path = PathBuf::from(trimmed);
    let absolute = if path.is_absolute() {
        path
    } else {
        let cwd = std::env::current_dir().map_err(|e| {
            ResumeStoreError::InvalidPath(format!("cannot resolve relative path: {e}"))
        })?;
        cwd.join(path)
    };
    let normalized = absolute
        .canonicalize()
        .unwrap_or_else(|_| normalize_components(&absolute));
    Ok(normalized.display().to_string())
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub fn sort_contexts_by_recency(contexts: &mut [ResumeContext]) {
    contexts.sort_by(|a, b| {
        let a_key = a
            .last_resumed_at
            .as_deref()
            .unwrap_or(a.updated_at.as_str());
        let b_key = b
            .last_resumed_at
            .as_deref()
            .unwrap_or(b.updated_at.as_str());
        b_key.cmp(a_key).then_with(|| a.name.cmp(&b.name))
    });
}

pub fn new_blank_context(name: &str) -> Result<ResumeContext, ResumeStoreError> {
    let key = normalize_name(name)?;
    let now = now_iso();
    Ok(ResumeContext {
        name: key.clone(),
        display_name: key,
        project_path: None,
        git_branch: None,
        worktree_path: None,
        ssh_host: None,
        recipes: Vec::new(),
        documents: Vec::new(),
        editor: None,
        editor_project_path: None,
        notes: Vec::new(),
        terminal_cwd: None,
        created_at: now.clone(),
        updated_at: now,
        last_resumed_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in_temp() -> (tempfile::TempDir, ResumeStore) {
        let dir = tempdir().unwrap();
        let store = ResumeStore::with_path(dir.path().join(RESUME_STORE_FILENAME)).unwrap();
        (dir, store)
    }

    #[test]
    fn serde_round_trip_and_unicode_paths() {
        let (_dir, store) = store_in_temp();
        let mut ctx = new_blank_context("luma").unwrap();
        ctx.project_path = Some("/Users/me/项目/Luma Next".into());
        ctx.git_branch = Some("feat/resume".into());
        ctx.worktree_path = Some("/Users/me/项目/Luma Next".into());
        ctx.documents = vec!["/Users/me/项目/Luma Next/README.md".into()];
        ctx.editor = Some(ResumeEditor::Cursor);
        ctx.editor_project_path = Some("/Users/me/项目/Luma Next".into());
        ctx.recipes = vec![ResumeRecipeRef {
            name: "check".into(),
            command: Some("cargo test".into()),
        }];
        store.upsert(ctx.clone()).unwrap();
        let loaded = store.get("luma").unwrap().unwrap();
        assert_eq!(loaded.project_path, ctx.project_path);
        assert_eq!(loaded.editor, Some(ResumeEditor::Cursor));
        assert_eq!(loaded.recipes[0].command.as_deref(), Some("cargo test"));
    }

    #[test]
    fn empty_missing_file_lists_empty() {
        let (_dir, store) = store_in_temp();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn corrupt_file_does_not_overwrite_original() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(RESUME_STORE_FILENAME);
        fs::write(&path, "{not-json").unwrap();
        let store = ResumeStore::with_path(path.clone()).unwrap();
        let err = store.list().unwrap_err();
        assert!(matches!(err, ResumeStoreError::Corrupt { .. }));
        let original = fs::read_to_string(&path).unwrap();
        assert_eq!(original, "{not-json");
        let backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("corrupt"))
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn upsert_get_delete_and_sort_by_last_resumed() {
        let (_dir, store) = store_in_temp();
        let mut a = new_blank_context("alpha").unwrap();
        a.updated_at = "2024-01-01T00:00:00Z".into();
        let mut b = new_blank_context("beta").unwrap();
        b.updated_at = "2024-06-01T00:00:00Z".into();
        store.upsert(a).unwrap();
        store.upsert(b).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list[0].name, "beta");
        store.mark_resumed("alpha").unwrap();
        let list = store.list().unwrap();
        assert_eq!(list[0].name, "alpha");
        assert!(list[0].last_resumed_at.is_some());
        store.delete("beta").unwrap();
        assert!(store.get("beta").unwrap().is_none());
    }

    #[test]
    fn path_with_spaces_normalizes_absolute() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("my project");
        fs::create_dir_all(&nested).unwrap();
        let normalized = normalize_path_for_store(nested.to_str().unwrap()).unwrap();
        assert!(normalized.contains("my project") || normalized.contains("my%20"));
        assert!(PathBuf::from(&normalized).is_absolute());
    }

    #[test]
    fn editor_parse_aliases() {
        assert_eq!(ResumeEditor::parse("VS Code"), Some(ResumeEditor::VsCode));
        assert_eq!(ResumeEditor::parse("idea"), Some(ResumeEditor::IntelliJ));
        assert_eq!(ResumeEditor::parse("cursor"), Some(ResumeEditor::Cursor));
    }

    #[test]
    fn rebuild_empty_after_corrupt() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(RESUME_STORE_FILENAME);
        fs::write(&path, "@@@").unwrap();
        let store = ResumeStore::with_path(path).unwrap();
        assert!(store.list().is_err());
        store.rebuild_empty().unwrap();
        assert!(store.list().unwrap().is_empty());
    }
}
