//! Filesystem-backed Notes workspace adapter.
//!
//! This is deliberately the only layer that resolves notes paths, touches directory entries, or
//! performs descriptor-relative note creation. The module receives vetted paths and presentation
//! data through `NotesWorkspacePort`.

use async_trait::async_trait;
use luma_application::{
    NotesDirectoryEntry, NotesDirectoryEntryKind, NotesDirectoryListing, NotesWorkspaceError,
    NotesWorkspacePath, NotesWorkspacePort, NotesWorkspacePreview,
};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

/// macOS implementation of the Notes workspace boundary.
pub struct MacNotesWorkspace;

const MAX_DIRECTORY_SCAN_MULTIPLIER: usize = 4;

#[async_trait]
impl NotesWorkspacePort for MacNotesWorkspace {
    async fn resolve_path(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        run_blocking(cancel, move |cancel| {
            resolve_path_sync(&root, &candidate, allow_root, &cancel)
        })
        .await
    }

    async fn existing_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        cancel: CancellationToken,
    ) -> Result<Option<NotesWorkspacePath>, NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        run_blocking(cancel, move |cancel| {
            let path = match prepare_open_sync(&root, &candidate, false, &cancel) {
                Ok(path) => path,
                Err(NotesWorkspaceError::NotFound) => return Ok(None),
                Err(error) => return Err(error),
            };
            let metadata = fs::symlink_metadata(&path.path).map_err(map_io_error)?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(NotesWorkspaceError::InvalidItem);
            }
            Ok(Some(path))
        })
        .await
    }

    async fn list_directory(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesDirectoryListing), NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        run_blocking(cancel, move |cancel| {
            list_directory_sync(&root, &candidate, max_entries, &cancel)
        })
        .await
    }

    async fn preview(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        max_bytes: usize,
        max_entries: usize,
        cancel: CancellationToken,
    ) -> Result<(NotesWorkspacePath, NotesWorkspacePreview), NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        run_blocking(cancel, move |cancel| {
            let path = prepare_open_sync(&root, &candidate, true, &cancel)?;
            let metadata = fs::symlink_metadata(&path.path).map_err(map_io_error)?;
            if metadata.file_type().is_symlink() {
                return Err(NotesWorkspaceError::InvalidItem);
            }
            if metadata.file_type().is_dir() {
                let (_, listing) = list_directory_sync(&root, &path.path, max_entries, &cancel)?;
                return Ok((path, NotesWorkspacePreview::Directory(listing)));
            }
            if !metadata.file_type().is_file() {
                return Ok((path, NotesWorkspacePreview::Unsupported));
            }
            let body = read_preview_no_follow(&path.path, max_bytes)?;
            Ok((path, NotesWorkspacePreview::File(body)))
        })
        .await
    }

    async fn create_note(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        body: String,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        // Once descriptor-relative creation begins it is a durable operation. Do not turn a
        // successfully created note into a late `Cancelled` response.
        run_blocking(cancel, move |cancel| {
            create_note_sync(&root, &candidate, &body, &cancel)
        })
        .await
    }

    async fn prepare_open(
        &self,
        root: PathBuf,
        candidate: PathBuf,
        allow_root: bool,
        cancel: CancellationToken,
    ) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        run_blocking(cancel, move |cancel| {
            prepare_open_sync(&root, &candidate, allow_root, &cancel)
        })
        .await
    }
}

async fn run_blocking<T, F>(
    cancel: CancellationToken,
    operation: F,
) -> Result<T, NotesWorkspaceError>
where
    T: Send + 'static,
    F: FnOnce(CancellationToken) -> Result<T, NotesWorkspaceError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || operation(cancel))
        .await
        .unwrap_or(Err(NotesWorkspaceError::Unavailable))
}

fn canonical_root(root: &Path) -> Result<PathBuf, NotesWorkspaceError> {
    let root = root
        .canonicalize()
        .map_err(|_| NotesWorkspaceError::RootUnavailable)?;
    let metadata = fs::symlink_metadata(&root).map_err(|_| NotesWorkspaceError::RootUnavailable)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(NotesWorkspaceError::RootUnavailable);
    }
    Ok(root)
}

fn resolve_path_sync(
    root: &Path,
    candidate: &Path,
    allow_root: bool,
    cancel: &CancellationToken,
) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
    if cancel.is_cancelled() {
        return Err(NotesWorkspaceError::Cancelled);
    }
    let root = canonical_root(root)?;
    resolve_path_from_root(&root, candidate, allow_root, cancel)
}

fn resolve_path_from_root(
    root: &Path,
    candidate: &Path,
    allow_root: bool,
    cancel: &CancellationToken,
) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        for component in candidate.components() {
            if matches!(component, Component::RootDir | Component::Prefix(_)) {
                return Err(NotesWorkspaceError::OutsideWorkspace);
            }
        }
        root.join(candidate)
    };

    // Canonicalize the longest existing ancestor. This resolves macOS aliases such as
    // `/var` → `/private/var` and detects a symlink that would otherwise escape the root.
    let mut existing = absolute.clone();
    let mut missing = Vec::<OsString>::new();
    loop {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = existing.file_name() else {
                    return Err(NotesWorkspaceError::NotFound);
                };
                missing.push(name.to_os_string());
                if !existing.pop() {
                    return Err(NotesWorkspaceError::NotFound);
                }
            }
            Err(_) => return Err(NotesWorkspaceError::Unavailable),
        }
    }
    let mut resolved = existing.canonicalize().map_err(map_io_error)?;
    if !resolved.starts_with(root) {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    for component in missing.into_iter().rev() {
        if component == ".." {
            return Err(NotesWorkspaceError::OutsideWorkspace);
        }
        resolved.push(component);
    }
    if !resolved.starts_with(root) {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    workspace_path(root, resolved, allow_root)
}

fn workspace_path(
    root: &Path,
    path: PathBuf,
    allow_root: bool,
) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| NotesWorkspaceError::OutsideWorkspace)?;
    if relative.as_os_str().is_empty() && !allow_root {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    let relative_path = relative.to_string_lossy().replace('\\', "/");
    Ok(NotesWorkspacePath {
        path,
        relative_path,
    })
}

fn prepare_open_sync(
    root: &Path,
    candidate: &Path,
    allow_root: bool,
    cancel: &CancellationToken,
) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
    let root = canonical_root(root)?;
    let path = resolve_path_from_root(&root, candidate, allow_root, cancel)?;
    let metadata = fs::symlink_metadata(&path.path).map_err(map_io_error)?;
    if metadata.file_type().is_symlink()
        || (!metadata.file_type().is_file() && !metadata.file_type().is_dir())
    {
        return Err(NotesWorkspaceError::InvalidItem);
    }
    // Resolve once more immediately before the external opener sees this path. The returned
    // canonical path avoids following a formerly in-root symlink after this validation.
    let canonical = path.path.canonicalize().map_err(map_io_error)?;
    if !canonical.starts_with(&root) {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    workspace_path(&root, canonical, allow_root)
}

fn list_directory_sync(
    root: &Path,
    candidate: &Path,
    max_entries: usize,
    cancel: &CancellationToken,
) -> Result<(NotesWorkspacePath, NotesDirectoryListing), NotesWorkspaceError> {
    let root = canonical_root(root)?;
    let directory = prepare_open_sync(&root, candidate, true, cancel)?;
    let metadata = fs::symlink_metadata(&directory.path).map_err(map_io_error)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(NotesWorkspaceError::InvalidItem);
    }
    let read_dir = fs::read_dir(&directory.path).map_err(map_io_error)?;
    let scan_limit = max_entries
        .saturating_mul(MAX_DIRECTORY_SCAN_MULTIPLIER)
        .max(max_entries.saturating_add(1));
    let mut scanned = 0usize;
    let mut listing = NotesDirectoryListing::default();

    for entry in read_dir {
        if cancel.is_cancelled() {
            return Err(NotesWorkspaceError::Cancelled);
        }
        if scanned >= scan_limit {
            listing.truncated = true;
            break;
        }
        scanned = scanned.saturating_add(1);
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        // Browse and previews deliberately do not enumerate symlinks, even if their target is
        // inside the workspace.
        if metadata.file_type().is_symlink() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let kind = if metadata.file_type().is_dir() {
            Some(NotesDirectoryEntryKind::Directory)
        } else if metadata.file_type().is_file() && name.to_ascii_lowercase().ends_with(".md") {
            Some(NotesDirectoryEntryKind::MarkdownFile)
        } else {
            None
        };
        let Some(kind) = kind else {
            continue;
        };
        let Ok(path) = resolve_path_from_root(&root, &path, true, cancel) else {
            continue;
        };
        if listing.entries.len() >= max_entries {
            listing.truncated = true;
            break;
        }
        listing.entries.push(NotesDirectoryEntry {
            name: name.to_string(),
            path,
            kind,
        });
    }
    listing.entries.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok((directory, listing))
}

fn create_note_sync(
    root: &Path,
    candidate: &Path,
    body: &str,
    cancel: &CancellationToken,
) -> Result<NotesWorkspacePath, NotesWorkspaceError> {
    if cancel.is_cancelled() {
        return Err(NotesWorkspaceError::Cancelled);
    }
    let root = canonical_root(root)?;
    let path = resolve_path_from_root(&root, candidate, false, cancel)?;
    let relative = path
        .path
        .strip_prefix(&root)
        .map_err(|_| NotesWorkspaceError::OutsideWorkspace)?;
    let components = relative_components(relative)?;
    create_new_file_no_follow(&root, &components, body)?;
    Ok(path)
}

fn relative_components(relative: &Path) -> Result<Vec<OsString>, NotesWorkspaceError> {
    let components: Vec<_> = relative
        .components()
        .map(|component| match component {
            Component::Normal(part) => Ok(part.to_os_string()),
            _ => Err(NotesWorkspaceError::OutsideWorkspace),
        })
        .collect::<Result<_, _>>()?;
    if components.is_empty() {
        return Err(NotesWorkspaceError::OutsideWorkspace);
    }
    Ok(components)
}

/// `O_NOFOLLOW` rejects a final symlink after the metadata check. `O_NONBLOCK` prevents a race
/// that swaps a regular file for a FIFO from blocking a preview before the opened descriptor is
/// checked.
#[cfg(target_os = "macos")]
const SAFE_READ_OPEN_FLAGS: i32 = 0x0104;
#[cfg(any(target_os = "linux", target_os = "android"))]
const SAFE_READ_OPEN_FLAGS: i32 = 0o404000;

fn read_preview_no_follow(path: &Path, max_bytes: usize) -> Result<String, NotesWorkspaceError> {
    let expected = fs::symlink_metadata(path).map_err(map_io_error)?;
    if expected.file_type().is_symlink() || !expected.file_type().is_file() {
        return Err(NotesWorkspaceError::InvalidItem);
    }
    let file = open_regular_non_symlink(path)?;
    let opened = file.metadata().map_err(map_io_error)?;
    if !opened.file_type().is_file() {
        return Err(NotesWorkspaceError::InvalidItem);
    }
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
    if expected.dev() != opened.dev() || expected.ino() != opened.ino() {
        return Err(NotesWorkspaceError::InvalidItem);
    }

    let mut bytes = Vec::with_capacity(max_bytes.min(8 * 1024));
    let mut reader = file.take(max_bytes as u64);
    reader.read_to_end(&mut bytes).map_err(map_io_error)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
fn open_regular_non_symlink(path: &Path) -> Result<File, NotesWorkspaceError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(SAFE_READ_OPEN_FLAGS)
        .open(path)
        .map_err(map_io_error)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "android")))]
fn open_regular_non_symlink(path: &Path) -> Result<File, NotesWorkspaceError> {
    // Luma itself is macOS-only; this fallback keeps tooling targets compilable after the
    // symlink metadata check above. Production uses `O_NOFOLLOW`.
    File::open(path).map_err(map_io_error)
}

#[cfg(unix)]
fn create_new_file_no_follow(
    root: &Path,
    components: &[OsString],
    body: &str,
) -> Result<(), NotesWorkspaceError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;

    #[cfg(target_os = "macos")]
    const O_DIRECTORY: i32 = 0x0010_0000;
    #[cfg(target_os = "macos")]
    const O_NOFOLLOW: i32 = 0x0100;
    #[cfg(target_os = "macos")]
    const O_CREAT: i32 = 0x0200;
    #[cfg(target_os = "macos")]
    const O_EXCL: i32 = 0x0800;
    #[cfg(all(unix, not(target_os = "macos")))]
    const O_DIRECTORY: i32 = 0o200000;
    #[cfg(all(unix, not(target_os = "macos")))]
    const O_NOFOLLOW: i32 = 0o400000;
    #[cfg(all(unix, not(target_os = "macos")))]
    const O_CREAT: i32 = 0o100;
    #[cfg(all(unix, not(target_os = "macos")))]
    const O_EXCL: i32 = 0o200;
    const O_RDONLY: i32 = 0;
    const O_WRONLY: i32 = 1;

    unsafe extern "C" {
        fn open(path: *const i8, oflag: i32, ...) -> i32;
        fn openat(dirfd: i32, path: *const i8, oflag: i32, ...) -> i32;
        fn mkdirat(dirfd: i32, path: *const i8, mode: u32) -> i32;
        fn unlinkat(dirfd: i32, path: *const i8, flags: i32) -> i32;
    }

    let root =
        CString::new(root.as_os_str().as_bytes()).map_err(|_| NotesWorkspaceError::Unavailable)?;
    // SAFETY: `root` is NUL-terminated and this adapter owns the returned descriptor.
    let root_fd = unsafe { open(root.as_ptr(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW) };
    if root_fd < 0 {
        return Err(NotesWorkspaceError::RootUnavailable);
    }
    // SAFETY: `root_fd` is a successful `open` result and is owned by this function.
    let mut directory = unsafe { OwnedFd::from_raw_fd(root_fd) };

    for component in &components[..components.len() - 1] {
        let name =
            CString::new(component.as_bytes()).map_err(|_| NotesWorkspaceError::Unavailable)?;
        // SAFETY: `directory` is a valid directory descriptor and `name` is NUL-terminated.
        let mut child = unsafe {
            openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
            )
        };
        if child < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(NotesWorkspaceError::Unavailable);
            }
            // SAFETY: same descriptor/name preconditions as `openat`; mode is explicit.
            if unsafe { mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o755) } != 0 {
                return Err(NotesWorkspaceError::Unavailable);
            }
            // Re-open through the parent descriptor with `O_NOFOLLOW`, so a concurrent symlink
            // swap cannot redirect the subsequent create.
            // SAFETY: same descriptor/name preconditions as above.
            child = unsafe {
                openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW,
                )
            };
            if child < 0 {
                return Err(NotesWorkspaceError::Unavailable);
            }
        }
        // SAFETY: `child` is a successful `openat` descriptor now owned by `directory`.
        directory = unsafe { OwnedFd::from_raw_fd(child) };
    }

    let name = CString::new(components[components.len() - 1].as_bytes())
        .map_err(|_| NotesWorkspaceError::Unavailable)?;
    // SAFETY: same descriptor/name preconditions as above. `O_EXCL` means an existing note is
    // never opened or overwritten.
    let file_fd = unsafe {
        openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW,
            0o644,
        )
    };
    if file_fd < 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            NotesWorkspaceError::AlreadyExists
        } else {
            NotesWorkspaceError::Unavailable
        });
    }
    // SAFETY: `file_fd` is a successful `openat` descriptor and becomes owned by `file`.
    let mut file = unsafe { File::from_raw_fd(file_fd) };
    if file
        .write_all(body.as_bytes())
        .and_then(|_| file.sync_all())
        .is_err()
    {
        drop(file);
        // Best-effort rollback of the new file. The name is still addressed through its original
        // directory descriptor, so this cannot follow a swapped parent symlink.
        // SAFETY: same descriptor/name preconditions as above; `flags = 0` removes a file only.
        let _ = unsafe { unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
        return Err(NotesWorkspaceError::Unavailable);
    }
    Ok(())
}

#[cfg(not(unix))]
fn create_new_file_no_follow(
    _root: &Path,
    _components: &[OsString],
    _body: &str,
) -> Result<(), NotesWorkspaceError> {
    Err(NotesWorkspaceError::Unavailable)
}

fn map_io_error(error: std::io::Error) -> NotesWorkspaceError {
    match error.kind() {
        std::io::ErrorKind::NotFound => NotesWorkspaceError::NotFound,
        std::io::ErrorKind::AlreadyExists => NotesWorkspaceError::AlreadyExists,
        _ => NotesWorkspaceError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn create_is_contained_and_never_overwrites_existing_note() {
        let dir = tempdir().unwrap();
        let workspace = MacNotesWorkspace;
        let created = workspace
            .create_note(
                dir.path().to_path_buf(),
                PathBuf::from("Inbox/first.md"),
                "# New note\n".into(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(fs::read_to_string(&created.path).unwrap(), "# New note\n");

        let error = workspace
            .create_note(
                dir.path().to_path_buf(),
                PathBuf::from("Inbox/first.md"),
                "replacement".into(),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error, NotesWorkspaceError::AlreadyExists);
        assert_eq!(fs::read_to_string(&created.path).unwrap(), "# New note\n");

        let escape = workspace
            .create_note(
                dir.path().to_path_buf(),
                PathBuf::from("../outside.md"),
                "no".into(),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(escape, NotesWorkspaceError::OutsideWorkspace);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn browse_and_preview_never_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let secret = outside.path().join("secret.md");
        fs::write(&secret, "TOPSECRET").unwrap();
        fs::write(dir.path().join("safe.md"), "# safe\n").unwrap();
        symlink(&secret, dir.path().join("secret.md")).unwrap();

        let workspace = MacNotesWorkspace;
        let (_, listing) = workspace
            .list_directory(
                dir.path().to_path_buf(),
                dir.path().to_path_buf(),
                32,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(listing.entries.iter().any(|entry| entry.name == "safe.md"));
        assert!(listing
            .entries
            .iter()
            .all(|entry| entry.name != "secret.md"));

        let error = workspace
            .preview(
                dir.path().to_path_buf(),
                dir.path().join("secret.md"),
                4_096,
                32,
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            NotesWorkspaceError::OutsideWorkspace | NotesWorkspaceError::InvalidItem
        ));
    }

    #[tokio::test]
    async fn preview_is_bounded_and_directory_results_are_sorted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("zeta.md"), "abcdefgh").unwrap();
        fs::write(dir.path().join("Alpha.md"), "# alpha\n").unwrap();
        fs::create_dir(dir.path().join("beta")).unwrap();
        let workspace = MacNotesWorkspace;

        let (_, preview) = workspace
            .preview(
                dir.path().to_path_buf(),
                dir.path().join("zeta.md"),
                4,
                32,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(preview, NotesWorkspacePreview::File("abcd".into()));

        let (_, listing) = workspace
            .list_directory(
                dir.path().to_path_buf(),
                dir.path().to_path_buf(),
                32,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        let names: Vec<_> = listing
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha.md", "beta", "zeta.md"]);
    }
}
