use luma_application::ProfileStoreError;
use serde::Serialize;
use serde_yaml::Value;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::parse::sequence_len;
use super::MAX_PROFILE_BYTES;

static ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn io_error(_: std::io::Error) -> ProfileStoreError {
    ProfileStoreError::Unavailable("Profile storage is unavailable".into())
}

pub(super) fn unsupported_schema() -> ProfileStoreError {
    ProfileStoreError::Unsupported("当前 Clash Verge Profile 结构暂不支持自动写回".into())
}

pub(super) fn safe_child(root: &Path, name: &str) -> Result<PathBuf, ProfileStoreError> {
    if name.contains('/') || name.contains('\\') || name == ".." {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile path escapes its controlled directory".into(),
        ));
    }
    let path = root.join(name);
    ensure_contained(root, &path)?;
    Ok(path)
}

pub(super) fn read_profile_stats(root: &Path, file: &str) -> Option<(usize, usize, usize)> {
    let path = safe_relative_child(root, file).ok()?;
    let value = read_yaml_file(&path).ok()?;
    Some((
        sequence_len(value.get("proxies")),
        sequence_len(value.get("proxy-groups")),
        sequence_len(value.get("rules")),
    ))
}

pub(super) fn safe_relative_child(root: &Path, file: &str) -> Result<PathBuf, ProfileStoreError> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ProfileStoreError::SecurityDenied(
            "Clash Verge Profile path escapes its controlled directory".into(),
        ));
    }
    let path = root.join(relative);
    ensure_contained(root, &path)?;
    Ok(path)
}

pub(super) fn ensure_contained(root: &Path, path: &Path) -> Result<(), ProfileStoreError> {
    let root = root.canonicalize().map_err(|_| {
        ProfileStoreError::SecurityDenied("Profile directory is unavailable".into())
    })?;
    if let Ok(existing) = path.canonicalize() {
        if !existing.starts_with(&root) {
            return Err(ProfileStoreError::SecurityDenied(
                "Profile path escapes its controlled directory".into(),
            ));
        }
    } else if path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .is_some_and(|p| !p.starts_with(&root))
    {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile path escapes its controlled directory".into(),
        ));
    }
    Ok(())
}
pub(super) fn canonical_local_file(path: &Path) -> Result<PathBuf, ProfileStoreError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| {
                ProfileStoreError::Unavailable("local Profile path is unavailable".into())
            })?
            .join(path)
    };
    let mut cur = PathBuf::new();
    for c in absolute.components() {
        match c {
            Component::RootDir => cur.push("/"),
            Component::Normal(part) => {
                cur.push(part);
                if fs::symlink_metadata(&cur)
                    .map_err(|_| ProfileStoreError::NotFound("local Profile file".into()))?
                    .file_type()
                    .is_symlink()
                    && !is_system_alias(&cur)
                {
                    return Err(ProfileStoreError::SecurityDenied(
                        "symbolic links are not allowed for local Profiles".into(),
                    ));
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                cur.pop();
            }
            Component::Prefix(p) => cur.push(p.as_os_str()),
        }
    }
    let canonical = absolute
        .canonicalize()
        .map_err(|_| ProfileStoreError::NotFound("local Profile file".into()))?;
    if !canonical.is_file() {
        return Err(ProfileStoreError::InvalidInput {
            field: "path".into(),
            message: "local Profile path is not a file".into(),
        });
    }
    Ok(canonical)
}
pub(super) fn read_yaml_file(path: &Path) -> Result<Value, ProfileStoreError> {
    let meta = fs::metadata(path).map_err(io_error)?;
    if meta.len() > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "Profile metadata exceeds the size limit".into(),
        ));
    }
    let raw = fs::read_to_string(path).map_err(io_error)?;
    serde_yaml::from_str(&raw).map_err(|_| unsupported_schema())
}
pub(super) fn atomic_write(
    path: &Path,
    bytes: &[u8],
    backup: bool,
) -> Result<(), ProfileStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    if backup && path.exists() {
        let backup_path = path.with_file_name(format!(
            "{}.bak",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("profile")
        ));
        fs::copy(path, &backup_path).map_err(io_error)?;
        set_private_file_mode(&backup_path)?;
    }
    let tmp = write_private_temp(path, bytes)?;
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(io_error(error));
    }
    set_private_file_mode(path)?;
    // The rename is the atomic commit. Syncing the containing directory is best effort: a
    // failure after a successful rename must not make the caller retry a completed transaction.
    if let Some(parent) = path.parent() {
        let _ = fs::File::open(parent).and_then(|directory| directory.sync_all());
    }
    Ok(())
}

/// Write a same-directory, private temporary file without reusing the old fixed `.tmp` name.
/// A per-process counter plus pid also prevents concurrent Luma processes from clobbering one
/// another's staging file. The eventual rename remains atomic because the file stays in `path`'s
/// directory.
fn write_private_temp(path: &Path, bytes: &[u8]) -> Result<PathBuf, ProfileStoreError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ProfileStoreError::Unavailable("Profile storage path is invalid".into()))?;
    for _ in 0..16 {
        let sequence = ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&tmp) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(error)),
        };
        let result = file.write_all(bytes).and_then(|_| file.sync_all());
        drop(file);
        if let Err(error) = result {
            let _ = fs::remove_file(&tmp);
            return Err(io_error(error));
        }
        // Keep the explicit mode check for platforms where the requested creation mode is
        // filtered or ignored. On macOS this also makes test and restore writes consistent.
        if let Err(error) = set_private_file_mode(&tmp) {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        return Ok(tmp);
    }
    Err(ProfileStoreError::Unavailable(
        "Profile storage could not allocate a temporary file".into(),
    ))
}
pub(super) fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ProfileStoreError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| {
        ProfileStoreError::Unavailable("Profile metadata could not be encoded".into())
    })?;
    atomic_write(path, &bytes, true)
}
pub(super) fn atomic_yaml(
    path: &Path,
    value: &Value,
    backup: bool,
) -> Result<(), ProfileStoreError> {
    let bytes = serde_yaml::to_string(value).map_err(|_| unsupported_schema())?;
    atomic_write(path, bytes.as_bytes(), backup)
}

pub(super) fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, ProfileStoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

pub(super) fn remove_file_if_exists(path: &Path) -> Result<(), ProfileStoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

pub(super) fn restore_file(path: &Path, bytes: Option<&[u8]>) -> Result<(), ProfileStoreError> {
    match bytes {
        Some(bytes) => atomic_write(path, bytes, false),
        None => remove_file_if_exists(path),
    }
}

pub(super) fn rollback_failure(
    original: ProfileStoreError,
    rollback: Result<(), ProfileStoreError>,
) -> ProfileStoreError {
    match rollback {
        Ok(()) => original,
        Err(error) => ProfileStoreError::Conflict(format!("{original}; rollback failed: {error}")),
    }
}

pub(super) fn combine_rollbacks(
    first: Result<(), ProfileStoreError>,
    second: Result<(), ProfileStoreError>,
) -> Result<(), ProfileStoreError> {
    match (first, second) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

#[cfg(unix)]
pub(super) fn set_private_file_mode(path: &Path) -> Result<(), ProfileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)
}

#[cfg(unix)]
pub(super) fn set_private_dir_mode(path: &Path) -> Result<(), ProfileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(io_error)
}

#[cfg(not(unix))]
pub(super) fn set_private_dir_mode(_path: &Path) -> Result<(), ProfileStoreError> {
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn set_private_file_mode(_path: &Path) -> Result<(), ProfileStoreError> {
    Ok(())
}

fn is_system_alias(path: &Path) -> bool {
    #[cfg(unix)]
    {
        matches!(path, p if p == Path::new("/tmp") || p == Path::new("/var") || p == Path::new("/etc"))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn atomic_write_does_not_reuse_a_legacy_fixed_temp_path() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("profile.yaml");
        // Older builds used `profile.tmp`; a stale directory there made every later write fail.
        let legacy_temp = target.with_extension("tmp");
        fs::create_dir(&legacy_temp).unwrap();

        atomic_write(&target, b"proxies: []\n", false).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"proxies: []\n");
        assert!(legacy_temp.is_dir());
    }
}
