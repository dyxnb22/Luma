//! macOS filesystem adapter for Projects workspace operations.
//!
//! The adapter owns path canonicalization, containment, and symlink checks. Directory work runs
//! on Tokio's blocking pool and has both an entry and a scan bound.

use async_trait::async_trait;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub use luma_application::{
    ProjectDirectoryEntry, ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError,
    ProjectWorkspacePort as ProjectWorkspace,
};

/// Maximum visible, non-hidden entries retained from a project directory.
pub const MAX_DIRECTORY_ENTRIES: usize = 512;
/// Limits filesystem work even when a directory has mostly hidden or unreadable children.
const MAX_DIRECTORY_SCAN: usize = 2_048;

pub struct MacProjectWorkspace;

#[async_trait]
impl ProjectWorkspace for MacProjectWorkspace {
    async fn resolve_import_path(
        &self,
        requested: PathBuf,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        run_blocking(cancel, move |cancel| {
            resolve_import_path_blocking(&requested, &cancel)
        })
        .await
    }

    async fn resolve_browse_path(
        &self,
        requested: PathBuf,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        run_blocking(cancel, move |cancel| {
            resolve_under_roots_blocking(&requested, &roots, &cancel)
        })
        .await
    }

    async fn list_directory(
        &self,
        directory: PathBuf,
        cancel: CancellationToken,
    ) -> Result<ProjectDirectoryListing, ProjectWorkspaceError> {
        run_blocking(cancel, move |cancel| {
            list_directory_blocking(&directory, &cancel)
        })
        .await
    }

    async fn imported_project_statuses(
        &self,
        paths: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<Vec<bool>, ProjectWorkspaceError> {
        run_blocking(cancel, move |cancel| {
            let mut statuses = Vec::with_capacity(paths.len());
            for path in paths {
                ensure_not_cancelled(&cancel)?;
                // Treat a replaced or manually configured symlink as unavailable. Import paths
                // are directories, and `resolve_open_path` validates again before Finder opens.
                statuses.push(matches!(
                    fs::symlink_metadata(path),
                    Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink()
                ));
            }
            Ok(statuses)
        })
        .await
    }

    async fn resolve_open_path(
        &self,
        requested: PathBuf,
        scope: ProjectOpenScope,
        roots: Vec<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<PathBuf, ProjectWorkspaceError> {
        run_blocking(cancel, move |cancel| {
            let resolved = match scope {
                ProjectOpenScope::ImportedProject => {
                    resolve_import_path_blocking(&requested, &cancel)?
                }
                ProjectOpenScope::ProjectRoots => {
                    resolve_under_roots_blocking(&requested, &roots, &cancel)?
                }
            };
            ensure_not_cancelled(&cancel)?;
            let metadata = fs::symlink_metadata(&resolved)
                .map_err(|error| workspace_io_error("inspect project path", error))?;
            if metadata.file_type().is_symlink() {
                return Err(ProjectWorkspaceError::Denied(
                    "refusing to open symlink".into(),
                ));
            }
            Ok(resolved)
        })
        .await
    }
}

async fn run_blocking<T, F>(cancel: CancellationToken, work: F) -> Result<T, ProjectWorkspaceError>
where
    T: Send + 'static,
    F: FnOnce(CancellationToken) -> Result<T, ProjectWorkspaceError> + Send + 'static,
{
    ensure_not_cancelled(&cancel)?;
    tokio::task::spawn_blocking(move || work(cancel))
        .await
        .map_err(|_| ProjectWorkspaceError::Unavailable("project workspace task stopped".into()))?
}

fn ensure_not_cancelled(cancel: &CancellationToken) -> Result<(), ProjectWorkspaceError> {
    if cancel.is_cancelled() {
        Err(ProjectWorkspaceError::Cancelled)
    } else {
        Ok(())
    }
}

fn resolve_import_path_blocking(
    requested: &Path,
    cancel: &CancellationToken,
) -> Result<PathBuf, ProjectWorkspaceError> {
    ensure_not_cancelled(cancel)?;
    let absolute = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| workspace_io_error("resolve current directory", error))?
            .join(requested)
    };

    // Inspect every component before canonicalizing. This rejects a symlink ancestor rather
    // than merely noticing where its final target lands. `..` is still accepted for ordinary
    // paths, but a preceding symlink has already been rejected before it can be popped.
    let mut current = PathBuf::new();
    for component in absolute.components() {
        ensure_not_cancelled(cancel)?;
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            Component::Normal(name) => {
                current.push(name);
                let metadata = fs::symlink_metadata(&current)
                    .map_err(|error| workspace_io_error("inspect project path", error))?;
                if metadata.file_type().is_symlink() && !is_macos_system_alias(&current) {
                    return Err(ProjectWorkspaceError::Denied("symlink not allowed".into()));
                }
            }
        }
    }

    ensure_not_cancelled(cancel)?;
    let metadata = fs::symlink_metadata(&absolute)
        .map_err(|error| workspace_io_error("inspect project path", error))?;
    if metadata.file_type().is_symlink() && !is_macos_system_alias(&absolute) {
        return Err(ProjectWorkspaceError::Denied("symlink not allowed".into()));
    }
    if !metadata.is_dir() {
        return Err(ProjectWorkspaceError::Denied(
            "path is not a directory".into(),
        ));
    }
    fs::canonicalize(&absolute).map_err(|error| workspace_io_error("resolve project path", error))
}

fn resolve_under_roots_blocking(
    requested: &Path,
    roots: &[PathBuf],
    cancel: &CancellationToken,
) -> Result<PathBuf, ProjectWorkspaceError> {
    if requested
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ProjectWorkspaceError::Denied(
            "path escapes project roots (..)".into(),
        ));
    }
    if roots.is_empty() {
        return Err(ProjectWorkspaceError::Unavailable(
            "no accessible project roots".into(),
        ));
    }

    let mut canonical_roots = Vec::new();
    for root in roots {
        ensure_not_cancelled(cancel)?;
        if let Ok(canonical) = fs::canonicalize(root) {
            canonical_roots.push(canonical);
        }
    }
    if canonical_roots.is_empty() {
        return Err(ProjectWorkspaceError::Unavailable(
            "no accessible project roots".into(),
        ));
    }

    let candidates = if requested.is_absolute() {
        vec![requested.to_path_buf()]
    } else {
        roots.iter().map(|root| root.join(requested)).collect()
    };
    let mut last_error = ProjectWorkspaceError::Denied("path escapes project roots".into());
    for candidate in candidates {
        ensure_not_cancelled(cancel)?;
        match resolve_candidate_under_roots(&candidate, &canonical_roots, cancel) {
            Ok(resolved) => return Ok(resolved),
            Err(ProjectWorkspaceError::Cancelled) => return Err(ProjectWorkspaceError::Cancelled),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn resolve_candidate_under_roots(
    requested: &Path,
    canonical_roots: &[PathBuf],
    cancel: &CancellationToken,
) -> Result<PathBuf, ProjectWorkspaceError> {
    let mut existing = requested.to_path_buf();
    let mut missing = Vec::<OsString>::new();
    let metadata = loop {
        ensure_not_cancelled(cancel)?;
        match fs::symlink_metadata(&existing) {
            Ok(metadata) => break metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => match existing.file_name() {
                Some(name) => {
                    missing.push(name.to_os_string());
                    if !existing.pop() {
                        return Err(ProjectWorkspaceError::NotFound(
                            "path has no existing ancestor under project roots".into(),
                        ));
                    }
                }
                None => {
                    return Err(ProjectWorkspaceError::NotFound(
                        "path has no existing ancestor under project roots".into(),
                    ));
                }
            },
            Err(error) => return Err(workspace_io_error("inspect project path", error)),
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(ProjectWorkspaceError::Denied(
            "symlink not allowed under project browse".into(),
        ));
    }

    let mut resolved = fs::canonicalize(&existing)
        .map_err(|error| workspace_io_error("resolve project path", error))?;
    if !is_under_roots(&resolved, canonical_roots) {
        return Err(ProjectWorkspaceError::Denied(
            "path escapes project roots".into(),
        ));
    }

    for component in missing.into_iter().rev() {
        ensure_not_cancelled(cancel)?;
        if component == OsStr::new("..") {
            return Err(ProjectWorkspaceError::Denied(
                "path escapes project roots (..)".into(),
            ));
        }
        if component == OsStr::new(".") {
            continue;
        }
        resolved.push(component);
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(ProjectWorkspaceError::Denied(
                        "symlink not allowed under project browse".into(),
                    ));
                }
                resolved = fs::canonicalize(&resolved)
                    .map_err(|error| workspace_io_error("resolve project path", error))?;
                if !is_under_roots(&resolved, canonical_roots) {
                    return Err(ProjectWorkspaceError::Denied(
                        "path escapes project roots".into(),
                    ));
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if !is_under_roots(&resolved, canonical_roots) {
                    return Err(ProjectWorkspaceError::Denied(
                        "path escapes project roots".into(),
                    ));
                }
            }
            Err(error) => return Err(workspace_io_error("inspect project path", error)),
        }
    }
    Ok(resolved)
}

fn list_directory_blocking(
    directory: &Path,
    cancel: &CancellationToken,
) -> Result<ProjectDirectoryListing, ProjectWorkspaceError> {
    ensure_not_cancelled(cancel)?;
    let metadata = fs::symlink_metadata(directory)
        .map_err(|error| workspace_io_error("inspect project directory", error))?;
    if metadata.file_type().is_symlink() {
        return Err(ProjectWorkspaceError::Denied(
            "symlink not allowed under project browse".into(),
        ));
    }
    if !metadata.is_dir() {
        return Err(ProjectWorkspaceError::NotFound(
            "project path is not a directory".into(),
        ));
    }
    let canonical_directory = fs::canonicalize(directory)
        .map_err(|error| workspace_io_error("resolve project directory", error))?;
    let entries = fs::read_dir(&canonical_directory)
        .map_err(|error| workspace_io_error("read project directory", error))?;

    let mut visible = Vec::new();
    let mut truncated = false;
    for (scanned, entry) in entries.enumerate() {
        ensure_not_cancelled(cancel)?;
        if scanned >= MAX_DIRECTORY_SCAN {
            truncated = true;
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        // Do not reveal or traverse links while browsing a project root.
        if metadata.file_type().is_symlink() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        if name.starts_with('.') {
            continue;
        }
        if visible.len() >= MAX_DIRECTORY_ENTRIES {
            truncated = true;
            break;
        }
        let Ok(canonical_path) = fs::canonicalize(&path) else {
            continue;
        };
        // Defend against a concurrent directory replacement between metadata and canonicalize.
        if !canonical_path.starts_with(&canonical_directory) {
            continue;
        }
        visible.push(ProjectDirectoryEntry {
            name,
            path: canonical_path,
            is_directory: metadata.is_dir(),
        });
    }
    visible.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(ProjectDirectoryListing {
        entries: visible,
        truncated,
    })
}

fn is_under_roots(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

fn workspace_io_error(action: &str, error: std::io::Error) -> ProjectWorkspaceError {
    if error.kind() == ErrorKind::NotFound {
        ProjectWorkspaceError::NotFound("path does not exist".into())
    } else {
        ProjectWorkspaceError::Unavailable(format!("could not {action}"))
    }
}

/// macOS exposes `/tmp`, `/var`, and `/etc` as aliases into `/private`.
/// They are OS aliases rather than user-controlled project symlinks; nested links remain denied.
#[cfg(unix)]
fn is_macos_system_alias(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !matches!(name, "tmp" | "var" | "etc") {
        return false;
    }
    let expected = Path::new("/private").join(name);
    path.is_absolute() && fs::canonicalize(path).ok().as_deref() == Some(expected.as_path())
}

#[cfg(not(unix))]
fn is_macos_system_alias(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn directory_listing_skips_symlinks_and_sorts_directories_first() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir(root.path().join("Folder")).unwrap();
        fs::write(root.path().join("readme.md"), "x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();

        let listing = MacProjectWorkspace
            .list_directory(root.path().to_path_buf(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            listing
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["Folder", "readme.md"]
        );
        assert!(listing.entries[0].is_directory);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn browse_and_import_reject_symlink_traversal() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::create_dir(outside.path().join("project")).unwrap();
        let link = root.path().join("escape");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();

        let browse_error = MacProjectWorkspace
            .resolve_browse_path(
                link.clone(),
                vec![root.path().to_path_buf()],
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(matches!(browse_error, ProjectWorkspaceError::Denied(_)));

        let import_error = MacProjectWorkspace
            .resolve_import_path(link.join("project"), CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(import_error, ProjectWorkspaceError::Denied(_)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn imported_symlink_is_marked_unavailable() {
        let root = tempdir().unwrap();
        let target = tempdir().unwrap();
        let project = target.path().join("project");
        fs::create_dir(&project).unwrap();
        let link = root.path().join("project-link");
        std::os::unix::fs::symlink(&project, &link).unwrap();

        let statuses = MacProjectWorkspace
            .imported_project_statuses(vec![link], CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(statuses, vec![false]);
    }

    #[tokio::test]
    async fn directory_listing_is_bounded() {
        let root = tempdir().unwrap();
        for number in 0..=MAX_DIRECTORY_ENTRIES {
            fs::write(root.path().join(format!("file-{number:04}")), "x").unwrap();
        }

        let listing = MacProjectWorkspace
            .list_directory(root.path().to_path_buf(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), MAX_DIRECTORY_ENTRIES);
        assert!(listing.truncated);
    }

    #[tokio::test]
    async fn cancellation_is_observed_before_directory_work() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = MacProjectWorkspace
            .list_directory(PathBuf::from("/does/not/exist"), cancel)
            .await
            .unwrap_err();
        assert_eq!(error, ProjectWorkspaceError::Cancelled);
    }
}
