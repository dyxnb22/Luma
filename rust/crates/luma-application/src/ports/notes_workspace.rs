//! Workspace boundary for the Notes module.
//!
//! Notes search results carry file paths, so path containment, symlink handling, bounded reads,
//! and file creation belong behind this port instead of in the module. Implementations return
//! only vetted paths and presentation-safe metadata; operating-system errors and note contents
//! never become error strings.

use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum NotesWorkspaceError {
    #[error("notes workspace operation cancelled")]
    Cancelled,
    #[error("notes path is outside the workspace")]
    OutsideWorkspace,
    #[error("notes workspace root is unavailable")]
    RootUnavailable,
    #[error("notes item was not found")]
    NotFound,
    #[error("notes item is not a regular file or folder")]
    InvalidItem,
    #[error("notes item already exists")]
    AlreadyExists,
    #[error("notes workspace is unavailable")]
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesWorkspacePath {
    /// Canonical, containment-checked absolute path. It is safe to show as a local path.
    pub path: PathBuf,
    /// Slash-separated path relative to the canonical workspace root. Empty means the root.
    pub relative_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotesDirectoryEntryKind {
    Directory,
    MarkdownFile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesDirectoryEntry {
    pub name: String,
    pub path: NotesWorkspacePath,
    pub kind: NotesDirectoryEntryKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotesDirectoryListing {
    /// Entries are sorted case-insensitively by name.
    pub entries: Vec<NotesDirectoryEntry>,
    /// The adapter stopped once its explicit entry limit was reached.
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotesWorkspacePreview {
    File(String),
    Directory(NotesDirectoryListing),
    Unsupported,
}

/// Platform file-system operations needed by Notes.
///
/// Implementations must reject symlinks and workspace escapes, perform blocking work outside
/// Tokio workers, and check `cancel` before and during directory work. `create_note` must create
/// a new file atomically, never overwrite an existing note, and remove a partially written new
/// file if its write fails.
#[async_trait]
pub trait NotesWorkspacePort: Send + Sync {
    /// Resolve a path under `root`; `allow_root` is only for workspace browse operations.
    async fn resolve_path(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError>;

    /// Returns a safe regular Markdown-note path when it exists, `None` when absent.
    async fn existing_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        cancel: CancellationToken,
    ) -> Result<Option<NotesWorkspacePath>, NotesWorkspaceError>;

    /// List immediate non-hidden directories and Markdown files without following symlinks.
    async fn list_directory(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesDirectoryListing), NotesWorkspaceError>;

    /// Read a safe file preview or directory listing. The file body is limited to `max_bytes`.
    async fn preview(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        max_bytes: usize,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesWorkspacePreview), NotesWorkspaceError>;

    /// Exclusively create a new note, sync its body, and clean up the new file if writing fails.
    /// Existing paths must never be overwritten.
    async fn create_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        body: String,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError>;

    /// Revalidate an existing non-symlink item immediately before an external open/copy action.
    async fn prepare_open(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError>;
}

/// In-memory Notes workspace for module tests. It never accesses the host filesystem.
#[derive(Default)]
pub struct FakeNotesWorkspace {
    listings: Arc<Mutex<BTreeMap<PathBuf, NotesDirectoryListing>>>,
    previews: Arc<Mutex<BTreeMap<PathBuf, NotesWorkspacePreview>>>,
    existing: Arc<Mutex<BTreeSet<PathBuf>>>,
    created: Arc<Mutex<Vec<(PathBuf, String)>>>,
    errors: Arc<Mutex<BTreeMap<PathBuf, NotesWorkspaceError>>>,
    queued_create_errors: Arc<Mutex<VecDeque<NotesWorkspaceError>>>,
}

impl FakeNotesWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_listing(&self, directory: impl Into<PathBuf>, listing: NotesDirectoryListing) {
        self.listings
            .lock()
            .expect("notes workspace fake lock")
            .insert(directory.into(), listing);
    }

    pub fn set_preview(&self, path: impl Into<PathBuf>, preview: NotesWorkspacePreview) {
        self.previews
            .lock()
            .expect("notes workspace fake lock")
            .insert(path.into(), preview);
    }

    pub fn mark_existing(&self, path: impl Into<PathBuf>) {
        self.existing
            .lock()
            .expect("notes workspace fake lock")
            .insert(path.into());
    }

    pub fn fail_path(&self, path: impl Into<PathBuf>, error: NotesWorkspaceError) {
        self.errors
            .lock()
            .expect("notes workspace fake lock")
            .insert(path.into(), error);
    }

    pub fn push_create_error(&self, error: NotesWorkspaceError) {
        self.queued_create_errors
            .lock()
            .expect("notes workspace fake lock")
            .push_back(error);
    }

    pub fn created_notes(&self) -> Vec<(PathBuf, String)> {
        self.created
            .lock()
            .expect("notes workspace fake lock")
            .clone()
    }

    fn cancelled(cancel: &CancellationToken) -> Result<(), NotesWorkspaceError> {
        if cancel.is_cancelled() {
            Err(NotesWorkspaceError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn resolved(
        root: &Path,
        candidate: PathBuf,
        allow_root: bool,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            if candidate.components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                return Err(NotesWorkspaceError::OutsideWorkspace);
            }
            root.join(candidate)
        };
        if !candidate.starts_with(root) {
            return Err(NotesWorkspaceError::OutsideWorkspace);
        }
        let relative = candidate
            .strip_prefix(root)
            .map_err(|_| NotesWorkspaceError::OutsideWorkspace)?;
        if relative.as_os_str().is_empty() && !allow_root {
            return Err(NotesWorkspaceError::OutsideWorkspace);
        }
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        Ok(NotesWorkspacePath {
            path: candidate,
            relative_path,
        })
    }

    fn error_for(&self, path: &Path) -> Option<NotesWorkspaceError> {
        self.errors
            .lock()
            .expect("notes workspace fake lock")
            .get(path)
            .cloned()
    }
}

#[async_trait]
impl NotesWorkspacePort for FakeNotesWorkspace {
    async fn resolve_path(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        Self::cancelled(&cancel)?;
        let resolved = Self::resolved(&root, candidate, allow_root)?;
        if let Some(error) = self.error_for(&resolved.path) {
            return Err(error);
        }
        Ok(resolved)
    }

    async fn existing_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        cancel: CancellationToken,
    ) -> Result<Option<NotesWorkspacePath>, NotesWorkspaceError> {
        let resolved = self.resolve_path(root, candidate, false, cancel).await?;
        Ok(self
            .existing
            .lock()
            .expect("notes workspace fake lock")
            .contains(&resolved.path)
            .then_some(resolved))
    }

    async fn list_directory(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesDirectoryListing), NotesWorkspaceError> {
        let directory = self.resolve_path(root, candidate, true, cancel).await?;
        let mut listing = self
            .listings
            .lock()
            .expect("notes workspace fake lock")
            .get(&directory.path)
            .cloned()
            .unwrap_or_default();
        if listing.entries.len() > max_entries {
            listing.entries.truncate(max_entries);
            listing.truncated = true;
        }
        Ok((directory, listing))
    }

    async fn preview(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        _max_bytes: usize,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesWorkspacePreview), NotesWorkspaceError> {
        let path = self.resolve_path(root, candidate, true, cancel).await?;
        let mut preview = self
            .previews
            .lock()
            .expect("notes workspace fake lock")
            .get(&path.path)
            .cloned()
            .unwrap_or(NotesWorkspacePreview::Unsupported);
        if let NotesWorkspacePreview::Directory(listing) = &mut preview {
            if listing.entries.len() > max_entries {
                listing.entries.truncate(max_entries);
                listing.truncated = true;
            }
        }
        Ok((path, preview))
    }

    async fn create_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        body: String,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        Self::cancelled(&cancel)?;
        let path = self
            .resolve_path(root, candidate, false, CancellationToken::new())
            .await?;
        if let Some(error) = self
            .queued_create_errors
            .lock()
            .expect("notes workspace fake lock")
            .pop_front()
        {
            return Err(error);
        }
        let mut existing = self.existing.lock().expect("notes workspace fake lock");
        if !existing.insert(path.path.clone()) {
            return Err(NotesWorkspaceError::AlreadyExists);
        }
        self.created
            .lock()
            .expect("notes workspace fake lock")
            .push((path.path.clone(), body));
        Ok(path)
    }

    async fn prepare_open(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        let path = self
            .resolve_path(root, candidate, allow_root, cancel)
            .await?;
        if allow_root && path.relative_path.is_empty() {
            return Ok(path);
        }
        if self
            .existing
            .lock()
            .expect("notes workspace fake lock")
            .contains(&path.path)
        {
            Ok(path)
        } else {
            Err(NotesWorkspaceError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_workspace_rejects_escape_without_host_io() {
        let fake = FakeNotesWorkspace::new();
        let error = fake
            .resolve_path(
                PathBuf::from("/workspace"),
                PathBuf::from("../outside.md"),
                false,
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error, NotesWorkspaceError::OutsideWorkspace);
    }

    #[tokio::test]
    async fn fake_workspace_records_new_note_body_without_filesystem() {
        let fake = FakeNotesWorkspace::new();
        let path = fake
            .create_note(
                PathBuf::from("/workspace"),
                PathBuf::from("Inbox/new.md"),
                "# New note\n".into(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(path.path, PathBuf::from("/workspace/Inbox/new.md"));
        assert_eq!(
            fake.created_notes(),
            vec![(path.path, "# New note\n".into())]
        );
    }
}
