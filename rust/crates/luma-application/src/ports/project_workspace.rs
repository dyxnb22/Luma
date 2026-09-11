//! Workspace boundary for the Projects module.
//!
//! Path resolution, symlink checks, and directory enumeration are platform I/O.
//! The module receives only vetted paths and directory rows through this port.

use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProjectWorkspaceError {
    #[error("project workspace operation cancelled")]
    Cancelled,
    #[error("project path denied: {0}")]
    Denied(String),
    #[error("project path not found: {0}")]
    NotFound(String),
    #[error("project workspace unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectDirectoryListing {
    pub entries: Vec<ProjectDirectoryEntry>,
    /// True when the adapter stopped enumeration at its safety bound.
    pub truncated: bool,
}

/// Defines how a path is authorized before Finder opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectOpenScope {
    ImportedProject,
    ProjectRoots,
}

/// Platform workspace operations needed by Projects.
///
/// Implementations must reject user-controlled symlink traversal and keep blocking file-system
/// work off Tokio workers. `cancel` is checked before and during directory work.
#[async_trait]
pub trait ProjectWorkspacePort: Send + Sync {
    async fn resolve_import_path(
        &self,
        requested: PathBuf,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError>;

    async fn resolve_browse_path(
        &self,
        requested: PathBuf,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError>;

    async fn list_directory(
        &self,
        directory: PathBuf,
        cancel: CancellationToken,
    ) -> Result<ProjectDirectoryListing, ProjectWorkspaceError>;

    /// Returns whether each configured imported-project path is currently present.
    async fn imported_project_statuses(
        &self,
        paths: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<Vec<bool>, ProjectWorkspaceError>;

    async fn resolve_open_path(
        &self,
        requested: PathBuf,
        scope: ProjectOpenScope,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError>;
}

/// In-memory workspace for module tests. It never reads the host filesystem.
#[derive(Default)]
pub struct FakeProjectWorkspace {
    listings: Arc<Mutex<BTreeMap<PathBuf, ProjectDirectoryListing>>>,
    missing_paths: Arc<Mutex<BTreeSet<PathBuf>>>,
    import_errors: Arc<Mutex<BTreeMap<PathBuf, ProjectWorkspaceError>>>,
    browse_errors: Arc<Mutex<BTreeMap<PathBuf, ProjectWorkspaceError>>>,
    open_errors: Arc<Mutex<BTreeMap<PathBuf, ProjectWorkspaceError>>>,
}

impl FakeProjectWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_listing(&self, directory: impl Into<PathBuf>, listing: ProjectDirectoryListing) {
        self.listings
            .lock()
            .expect("project workspace fake lock")
            .insert(directory.into(), listing);
    }

    pub fn mark_missing(&self, path: impl Into<PathBuf>) {
        self.missing_paths
            .lock()
            .expect("project workspace fake lock")
            .insert(path.into());
    }

    pub fn fail_import(&self, path: impl Into<PathBuf>, error: ProjectWorkspaceError) {
        self.import_errors
            .lock()
            .expect("project workspace fake lock")
            .insert(path.into(), error);
    }

    pub fn fail_browse(&self, path: impl Into<PathBuf>, error: ProjectWorkspaceError) {
        self.browse_errors
            .lock()
            .expect("project workspace fake lock")
            .insert(path.into(), error);
    }

    pub fn fail_open(&self, path: impl Into<PathBuf>, error: ProjectWorkspaceError) {
        self.open_errors
            .lock()
            .expect("project workspace fake lock")
            .insert(path.into(), error);
    }

    fn cancelled(cancel: &CancellationToken) -> Result<(), ProjectWorkspaceError> {
        if cancel.is_cancelled() {
            Err(ProjectWorkspaceError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn resolve_browse_default(
        requested: PathBuf,
        roots: &[PathBuf],
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        if requested
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(ProjectWorkspaceError::Denied(
                "path escapes project roots (..)".into(),
            ));
        }
        if requested.is_absolute() {
            if roots.iter().any(|root| requested.starts_with(root)) {
                return Ok(requested);
            }
        } else if let Some(root) = roots.first() {
            return Ok(root.join(requested));
        }
        Err(ProjectWorkspaceError::Denied(
            "path escapes project roots".into(),
        ))
    }
}

#[async_trait]
impl ProjectWorkspacePort for FakeProjectWorkspace {
    async fn resolve_import_path(
        &self,
        requested: PathBuf,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        Self::cancelled(&cancel)?;
        if let Some(error) = self
            .import_errors
            .lock()
            .expect("project workspace fake lock")
            .get(&requested)
            .cloned()
        {
            return Err(error);
        }
        Ok(requested)
    }

    async fn resolve_browse_path(
        &self,
        requested: PathBuf,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        Self::cancelled(&cancel)?;
        if let Some(error) = self
            .browse_errors
            .lock()
            .expect("project workspace fake lock")
            .get(&requested)
            .cloned()
        {
            return Err(error);
        }
        Self::resolve_browse_default(requested, &roots)
    }

    async fn list_directory(
        &self,
        directory: PathBuf,
        cancel: CancellationToken,
    ) -> Result<ProjectDirectoryListing, ProjectWorkspaceError> {
        Self::cancelled(&cancel)?;
        Ok(self
            .listings
            .lock()
            .expect("project workspace fake lock")
            .get(&directory)
            .cloned()
            .unwrap_or_default())
    }

    async fn imported_project_statuses(
        &self,
        paths: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<Vec<bool>, ProjectWorkspaceError> {
        Self::cancelled(&cancel)?;
        let missing = self
            .missing_paths
            .lock()
            .expect("project workspace fake lock")
            .clone();
        Ok(paths
            .into_iter()
            .map(|path| !missing.contains(&path))
            .collect())
    }

    async fn resolve_open_path(
        &self,
        requested: PathBuf,
        scope: ProjectOpenScope,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        Self::cancelled(&cancel)?;
        if let Some(error) = self
            .open_errors
            .lock()
            .expect("project workspace fake lock")
            .get(&requested)
            .cloned()
        {
            return Err(error);
        }
        match scope {
            ProjectOpenScope::ImportedProject => Ok(requested),
            ProjectOpenScope::ProjectRoots => Self::resolve_browse_default(requested, &roots),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_workspace_is_cancelled_without_host_io() {
        let fake = FakeProjectWorkspace::new();
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            fake.resolve_import_path(PathBuf::from("/does/not/exist"), cancel)
                .await,
            Err(ProjectWorkspaceError::Cancelled)
        );
    }
}
