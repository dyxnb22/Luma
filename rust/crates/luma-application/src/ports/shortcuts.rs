use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_SHORTCUT_RESULTS: usize = 500;
pub const MAX_SHORTCUT_OUTPUT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutEntry {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutRunPlan {
    pub program: String,
    pub args: Vec<String>,
    pub shortcut: ShortcutEntry,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ShortcutsError {
    #[error("Apple Shortcuts command is unavailable")]
    Unavailable,
    #[error("Shortcuts command failed: {0}")]
    CommandFailed(String),
    #[error("Shortcuts command timed out")]
    Timeout,
    #[error("Shortcuts output exceeded the {0}-byte limit")]
    OutputTooLarge(usize),
    #[error("shortcut not found")]
    NotFound,
    #[error("shortcut name is ambiguous")]
    Ambiguous,
    #[error("shortcut operation cancelled")]
    Cancelled,
}

#[async_trait]
pub trait ShortcutsPort: Send + Sync {
    async fn list(
        &self,
        folder: Option<&str>,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<ShortcutEntry>, ShortcutsError>;
    async fn folders(
        &self,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<String>, ShortcutsError>;
    async fn resolve_exact(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutEntry, ShortcutsError>;
    async fn view(&self, name: &str, cancel: CancellationToken) -> Result<(), ShortcutsError>;
    async fn run_plan(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutRunPlan, ShortcutsError>;
}

/// In-memory fake. It never invokes Shortcuts.
pub struct FakeShortcuts {
    entries: Mutex<Vec<ShortcutEntry>>,
    folders: Mutex<Vec<String>>,
    error: Mutex<Option<ShortcutsError>>,
    pub view_calls: Arc<Mutex<Vec<String>>>,
    pub run_calls: Arc<Mutex<Vec<String>>>,
}

impl FakeShortcuts {
    pub fn new(entries: Vec<ShortcutEntry>, folders: Vec<String>) -> Self {
        Self {
            entries: Mutex::new(entries),
            folders: Mutex::new(folders),
            error: Mutex::new(None),
            view_calls: Arc::new(Mutex::new(Vec::new())),
            run_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn fail_with(&self, error: ShortcutsError) {
        *self.error.lock().expect("shortcuts error lock") = Some(error);
    }

    pub fn replace(&self, entries: Vec<ShortcutEntry>) {
        *self.entries.lock().expect("shortcut entries lock") = entries;
    }

    fn take_error(&self) -> Result<(), ShortcutsError> {
        match self.error.lock().expect("shortcuts error lock").take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn exact(&self, name: &str) -> Result<ShortcutEntry, ShortcutsError> {
        let matches = self
            .entries
            .lock()
            .expect("shortcut entries lock")
            .iter()
            .filter(|entry| entry.name == name)
            .cloned()
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [entry] => Ok(entry.clone()),
            [] => Err(ShortcutsError::NotFound),
            _ => Err(ShortcutsError::Ambiguous),
        }
    }
}

#[async_trait]
impl ShortcutsPort for FakeShortcuts {
    async fn list(
        &self,
        _folder: Option<&str>,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<ShortcutEntry>, ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.take_error()?;
        let mut entries = self.entries.lock().expect("shortcut entries lock").clone();
        entries.truncate(limit.min(MAX_SHORTCUT_RESULTS));
        Ok(entries)
    }

    async fn folders(
        &self,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<String>, ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.take_error()?;
        let mut folders = self.folders.lock().expect("shortcut folders lock").clone();
        folders.truncate(limit.min(MAX_SHORTCUT_RESULTS));
        Ok(folders)
    }

    async fn resolve_exact(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutEntry, ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.take_error()?;
        self.exact(name)
    }

    async fn view(&self, name: &str, cancel: CancellationToken) -> Result<(), ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.take_error()?;
        self.exact(name)?;
        self.view_calls
            .lock()
            .expect("shortcut view calls lock")
            .push(name.into());
        Ok(())
    }

    async fn run_plan(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<ShortcutRunPlan, ShortcutsError> {
        if cancel.is_cancelled() {
            return Err(ShortcutsError::Cancelled);
        }
        self.take_error()?;
        let shortcut = self.exact(name)?;
        self.run_calls
            .lock()
            .expect("shortcut run calls lock")
            .push(name.into());
        Ok(ShortcutRunPlan {
            program: "/usr/bin/shortcuts".into(),
            args: vec!["run".into(), name.into()],
            shortcut,
        })
    }
}
