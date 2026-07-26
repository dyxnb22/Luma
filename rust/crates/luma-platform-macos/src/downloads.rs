//! Bounded, direct-child Downloads access. Filesystem mutation and Finder Trash calls stay here.

use async_trait::async_trait;
use luma_application::{
    DownloadCategory, DownloadEntry, DownloadsError, DownloadsFilter, DownloadsPort,
    MAX_DOWNLOAD_ENTRIES,
};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;

pub struct MacDownloads {
    root: PathBuf,
}

impl MacDownloads {
    pub fn system_default() -> Result<Self, DownloadsError> {
        let root = dirs::home_dir()
            .ok_or(DownloadsError::NotConfigured)?
            .join("Downloads");
        Ok(Self { root })
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    async fn canonical_root(&self) -> Result<PathBuf, DownloadsError> {
        match tokio::fs::canonicalize(&self.root).await {
            Ok(root) => {
                let metadata = tokio::fs::metadata(&root).await.map_err(map_root_error)?;
                if metadata.is_dir() {
                    Ok(root)
                } else {
                    Err(DownloadsError::NotConfigured)
                }
            }
            Err(error) => Err(map_root_error(error)),
        }
    }

    async fn scan(&self, cancel: &CancellationToken) -> Result<Vec<DownloadEntry>, DownloadsError> {
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        let root = self.canonical_root().await?;
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        let mut reader = tokio::fs::read_dir(&root)
            .await
            .map_err(|error| DownloadsError::Io(error.to_string()))?;
        let mut entries = Vec::new();
        let mut scanned = 0_usize;
        while scanned < MAX_DOWNLOAD_ENTRIES {
            if cancel.is_cancelled() {
                return Err(DownloadsError::Cancelled);
            }
            let Some(dir_entry) = reader
                .next_entry()
                .await
                .map_err(|error| DownloadsError::Io(error.to_string()))?
            else {
                break;
            };
            scanned += 1;
            let file_type = dir_entry
                .file_type()
                .await
                .map_err(|error| DownloadsError::Io(error.to_string()))?;
            if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
                continue;
            }
            let path = dir_entry.path();
            let metadata = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(|error| DownloadsError::Io(error.to_string()))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            entries.push(entry_from_metadata(&root, path, metadata));
        }
        Ok(entries)
    }

    async fn resolve_internal(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError> {
        let entry = self
            .scan(cancel)
            .await?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or(DownloadsError::NotFound)?;
        self.validate_parent(&entry.path).await?;
        Ok(entry)
    }

    async fn validate_parent(&self, path: &Path) -> Result<PathBuf, DownloadsError> {
        let root = self.canonical_root().await?;
        let parent = path.parent().ok_or_else(|| {
            DownloadsError::Invalid("download item has no parent directory".into())
        })?;
        let parent = tokio::fs::canonicalize(parent)
            .await
            .map_err(|error| DownloadsError::Io(error.to_string()))?;
        if parent != root {
            return Err(DownloadsError::Invalid(
                "download item escaped the configured Downloads root".into(),
            ));
        }
        Ok(root)
    }
}

#[async_trait]
impl DownloadsPort for MacDownloads {
    async fn list(
        &self,
        filter: DownloadsFilter,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<DownloadEntry>, DownloadsError> {
        let mut entries = self.scan(&cancel).await?;
        match filter {
            DownloadsFilter::Recent => sort_recent(&mut entries),
            DownloadsFilter::Large => {
                entries.retain(|entry| !entry.is_directory);
                entries.sort_by(|a, b| {
                    b.size_bytes
                        .cmp(&a.size_bytes)
                        .then_with(|| b.modified_unix.cmp(&a.modified_unix))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            DownloadsFilter::Old { days } => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .min(i64::MAX as u64) as i64;
                let cutoff = now.saturating_sub(i64::from(days) * 86_400);
                entries.retain(|entry| entry.modified_unix <= cutoff);
                entries.sort_by(|a, b| {
                    a.modified_unix
                        .cmp(&b.modified_unix)
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            DownloadsFilter::Type(category) => {
                entries.retain(|entry| entry.category == category);
                sort_recent(&mut entries);
            }
            DownloadsFilter::Text(needle) => {
                let needle = needle.to_lowercase();
                entries.retain(|entry| entry.display_name.to_lowercase().contains(&needle));
                sort_recent(&mut entries);
            }
        }
        entries.truncate(limit.min(MAX_DOWNLOAD_ENTRIES));
        Ok(entries)
    }

    async fn resolve(
        &self,
        id: &str,
        cancel: CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError> {
        self.resolve_internal(id, &cancel).await
    }

    async fn rename(
        &self,
        id: &str,
        expected_identity: &str,
        new_name: &str,
        cancel: CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError> {
        validate_new_name(new_name)?;
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        let current = self.resolve_internal(id, &cancel).await?;
        if current.identity != expected_identity {
            return Err(DownloadsError::StaleIdentity);
        }
        let root = self.validate_parent(&current.path).await?;
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        let destination = root.join(new_name);
        match tokio::fs::symlink_metadata(&destination).await {
            Ok(_) => return Err(DownloadsError::Conflict),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(DownloadsError::Io(error.to_string())),
        }
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        tokio::fs::rename(&current.path, &destination)
            .await
            .map_err(|error| DownloadsError::Io(error.to_string()))?;
        let metadata = tokio::fs::symlink_metadata(&destination)
            .await
            .map_err(|error| DownloadsError::Io(error.to_string()))?;
        Ok(entry_from_metadata(&root, destination, metadata))
    }

    async fn trash(
        &self,
        id: &str,
        expected_identity: &str,
        cancel: CancellationToken,
    ) -> Result<(), DownloadsError> {
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        let current = self.resolve_internal(id, &cancel).await?;
        if current.identity != expected_identity {
            return Err(DownloadsError::StaleIdentity);
        }
        self.validate_parent(&current.path).await?;
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        trash_with_finder(&current.path).await
    }
}

fn map_root_error(error: std::io::Error) -> DownloadsError {
    match error.kind() {
        std::io::ErrorKind::NotFound => DownloadsError::NotConfigured,
        std::io::ErrorKind::PermissionDenied => DownloadsError::Unavailable(
            "permission denied while reading the Downloads folder".into(),
        ),
        _ => DownloadsError::Io(error.to_string()),
    }
}

fn sort_recent(entries: &mut [DownloadEntry]) {
    entries.sort_by(|a, b| {
        b.modified_unix
            .cmp(&a.modified_unix)
            .then_with(|| a.display_name.cmp(&b.display_name))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn entry_from_metadata(root: &Path, path: PathBuf, metadata: std::fs::Metadata) -> DownloadEntry {
    let file_name = path.file_name().unwrap_or_default();
    let display_name = display_file_name(file_name);
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0);
    DownloadEntry {
        id: path_id(root, file_name.as_encoded_bytes()),
        identity: metadata_identity(&metadata),
        category: category_for_path(&path),
        display_name,
        path,
        size_bytes: metadata.len(),
        modified_unix,
        is_directory: metadata.is_dir(),
    }
}

fn display_file_name(file_name: &std::ffi::OsStr) -> String {
    file_name
        .to_string_lossy()
        .chars()
        .map(|ch| if ch.is_control() { '\u{fffd}' } else { ch })
        .collect()
}

fn path_id(root: &Path, name: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(root.as_os_str().as_encoded_bytes());
    digest.update([0]);
    digest.update(name);
    hex::encode(digest.finalize())
}

#[cfg(unix)]
fn metadata_identity(metadata: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "{}:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.len()
    )
}

#[cfg(not(unix))]
fn metadata_identity(metadata: &std::fs::Metadata) -> String {
    format!(
        "{}:{}",
        metadata.len(),
        metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    )
}

fn category_for_path(path: &Path) -> DownloadCategory {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => DownloadCategory::Archive,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "tiff" | "svg" => {
            DownloadCategory::Image
        }
        "mov" | "mp4" | "m4v" | "mkv" | "avi" | "webm" => DownloadCategory::Video,
        "pdf" | "txt" | "md" | "rtf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => {
            DownloadCategory::Document
        }
        "dmg" | "pkg" => DownloadCategory::Installer,
        _ => DownloadCategory::Other,
    }
}

fn validate_new_name(name: &str) -> Result<(), DownloadsError> {
    if name.is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
        return Err(DownloadsError::Invalid(
            "new name must be 1-255 bytes and contain no control characters".into(),
        ));
    }
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if name != "." && name != ".." => Ok(()),
        _ => Err(DownloadsError::Invalid(
            "new name must be one filename without separators".into(),
        )),
    }
}

fn finder_trash_args(path: &Path) -> Vec<OsString> {
    vec![
        "-e".into(),
        "on run argv".into(),
        "-e".into(),
        "tell application \"Finder\" to delete POSIX file (item 1 of argv)".into(),
        "-e".into(),
        "end run".into(),
        path.as_os_str().to_owned(),
    ]
}

async fn trash_with_finder(path: &Path) -> Result<(), DownloadsError> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(DownloadsError::Unavailable(
            "Finder Trash is available only on macOS".into(),
        ))
    }
    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("/usr/bin/osascript")
            .args(finder_trash_args(path))
            .status()
            .await
            .map_err(|error| DownloadsError::Unavailable(error.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(DownloadsError::Io(format!("Finder Trash exited {status}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn lists_direct_children_filters_and_skips_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("photo.jpg"), b"123").unwrap();
        fs::write(temp.path().join("archive.zip"), b"123456").unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/hidden.pdf"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            temp.path().join("archive.zip"),
            temp.path().join("linked.zip"),
        )
        .unwrap();
        let port = MacDownloads::with_root(temp.path().to_path_buf());

        let recent = port
            .list(DownloadsFilter::Recent, 500, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(recent.len(), 3);
        assert!(!recent
            .iter()
            .any(|entry| entry.display_name == "linked.zip"));
        assert!(!recent
            .iter()
            .any(|entry| entry.display_name == "hidden.pdf"));

        let large = port
            .list(DownloadsFilter::Large, 500, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(large[0].display_name, "archive.zip");

        let images = port
            .list(
                DownloadsFilter::Type(DownloadCategory::Image),
                500,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].display_name, "photo.jpg");

        let text = port
            .list(
                DownloadsFilter::Text("ARCH".into()),
                500,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(text.len(), 1);
    }

    #[tokio::test]
    async fn scan_is_capped_and_pre_cancelled_without_io() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..510 {
            fs::write(temp.path().join(format!("{index:03}.txt")), b"x").unwrap();
        }
        let port = MacDownloads::with_root(temp.path().to_path_buf());
        let entries = port
            .list(
                DownloadsFilter::Recent,
                usize::MAX,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(entries.len(), MAX_DOWNLOAD_ENTRIES);

        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            port.list(DownloadsFilter::Recent, 10, cancel).await,
            Err(DownloadsError::Cancelled)
        );
    }

    #[tokio::test]
    async fn rename_revalidates_identity_scope_collision_and_name() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("one.txt"), b"one").unwrap();
        fs::write(temp.path().join("taken.txt"), b"taken").unwrap();
        let port = MacDownloads::with_root(temp.path().to_path_buf());
        let entry = port
            .list(
                DownloadsFilter::Text("one".into()),
                10,
                CancellationToken::new(),
            )
            .await
            .unwrap()
            .remove(0);

        assert_eq!(
            port.rename(&entry.id, "stale", "two.txt", CancellationToken::new())
                .await,
            Err(DownloadsError::StaleIdentity)
        );
        assert_eq!(
            port.rename(
                &entry.id,
                &entry.identity,
                "../escape.txt",
                CancellationToken::new()
            )
            .await,
            Err(DownloadsError::Invalid(
                "new name must be one filename without separators".into()
            ))
        );
        assert_eq!(
            port.rename(
                &entry.id,
                &entry.identity,
                "taken.txt",
                CancellationToken::new()
            )
            .await,
            Err(DownloadsError::Conflict)
        );
        let renamed = port
            .rename(
                &entry.id,
                &entry.identity,
                "two.txt",
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(renamed.display_name, "two.txt");
        assert!(!temp.path().join("one.txt").exists());
        assert!(temp.path().join("two.txt").exists());
    }

    #[tokio::test]
    async fn replacement_is_detected_as_stale_before_rename() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("one.txt");
        fs::write(&path, b"one").unwrap();
        let port = MacDownloads::with_root(temp.path().to_path_buf());
        let entry = port
            .list(DownloadsFilter::Recent, 10, CancellationToken::new())
            .await
            .unwrap()
            .remove(0);
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"replacement").unwrap();
        assert_eq!(
            port.rename(
                &entry.id,
                &entry.identity,
                "two.txt",
                CancellationToken::new()
            )
            .await,
            Err(DownloadsError::StaleIdentity)
        );
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_filename_is_renderable_and_has_lossless_identity() {
        use std::os::unix::ffi::OsStringExt;
        let temp = tempfile::tempdir().unwrap();
        let valid = temp.path().join("metadata-source");
        fs::write(&valid, b"x").unwrap();
        let metadata = fs::metadata(valid).unwrap();
        let name = OsString::from_vec(vec![b'b', b'a', b'd', 0xff]);
        let entry = entry_from_metadata(temp.path(), temp.path().join(name), metadata);
        assert!(entry.display_name.contains('\u{fffd}'));
        assert_eq!(entry.id.len(), 64);
        assert!(!entry.identity.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn control_characters_never_enter_download_display_names() {
        use std::os::unix::ffi::OsStringExt;
        let name = OsString::from_vec(b"line\nbreak.txt".to_vec());
        let display = display_file_name(&name);
        assert_eq!(display, "line\u{fffd}break.txt");
    }

    #[test]
    fn finder_trash_uses_direct_argv_and_never_interpolates_path() {
        let hostile = Path::new("/tmp/quote'\"; shell");
        let args = finder_trash_args(hostile);
        assert_eq!(args.last().unwrap(), hostile.as_os_str());
        assert!(args[..args.len() - 1].iter().all(|arg| !arg
            .to_string_lossy()
            .contains(hostile.to_string_lossy().as_ref())));
    }
}
