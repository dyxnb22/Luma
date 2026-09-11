//! Safe local-file adapter for bounded module imports.

use async_trait::async_trait;
use luma_application::{BoundedUtf8FileReadError, BoundedUtf8FileReaderPort};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
use std::os::unix::fs::OpenOptionsExt;

/// macOS implementation of the bounded UTF-8 import reader port.
pub struct MacBoundedUtf8FileReader;

#[async_trait]
impl BoundedUtf8FileReaderPort for MacBoundedUtf8FileReader {
    async fn read_utf8(
        &self,
        path: &Path,
        max_bytes: usize,
    ) -> Result<String, BoundedUtf8FileReadError> {
        let path = path.to_path_buf();
        match tokio::task::spawn_blocking(move || read_bounded_utf8(&path, max_bytes)).await {
            Ok(result) => result,
            Err(_) => Err(BoundedUtf8FileReadError::Unavailable),
        }
    }
}

fn read_bounded_utf8(path: &Path, max_bytes: usize) -> Result<String, BoundedUtf8FileReadError> {
    let expected = fs::symlink_metadata(path).map_err(|_| BoundedUtf8FileReadError::Unavailable)?;
    if !expected.file_type().is_file() {
        return Err(BoundedUtf8FileReadError::InvalidFile);
    }
    if expected.len() > max_bytes as u64 {
        return Err(BoundedUtf8FileReadError::TooLarge);
    }

    let file = open_regular_non_symlink(path)?;
    let opened = file
        .metadata()
        .map_err(|_| BoundedUtf8FileReadError::Unavailable)?;
    if !opened.is_file() {
        return Err(BoundedUtf8FileReadError::InvalidFile);
    }
    #[cfg(unix)]
    if expected.dev() != opened.dev() || expected.ino() != opened.ino() {
        return Err(BoundedUtf8FileReadError::InvalidFile);
    }
    if opened.len() > max_bytes as u64 {
        return Err(BoundedUtf8FileReadError::TooLarge);
    }

    let mut bytes = Vec::new();
    let mut bounded = file.take((max_bytes as u64).saturating_add(1));
    bounded
        .read_to_end(&mut bytes)
        .map_err(|_| BoundedUtf8FileReadError::Unavailable)?;
    if bytes.len() > max_bytes {
        return Err(BoundedUtf8FileReadError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| BoundedUtf8FileReadError::InvalidUtf8)
}

// `O_NOFOLLOW` prevents following a final symlink after the metadata check. `O_NONBLOCK` also
// prevents a race that swaps a regular file for a FIFO from blocking the adapter before `fstat`.
#[cfg(target_os = "macos")]
const SAFE_READ_OPEN_FLAGS: i32 = 0x0104;
#[cfg(any(target_os = "linux", target_os = "android"))]
const SAFE_READ_OPEN_FLAGS: i32 = 0o404000;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
fn open_regular_non_symlink(path: &Path) -> Result<File, BoundedUtf8FileReadError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(SAFE_READ_OPEN_FLAGS)
        .open(path)
        .map_err(|_| BoundedUtf8FileReadError::Unavailable)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "android")))]
fn open_regular_non_symlink(path: &Path) -> Result<File, BoundedUtf8FileReadError> {
    // Luma itself is macOS-only. This fallback preserves compilation for tooling targets after
    // `symlink_metadata` has rejected a static symlink; production uses the no-follow path above.
    File::open(path).map_err(|_| BoundedUtf8FileReadError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn reads_regular_utf8_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("words.csv");
        fs::write(&path, "term,meaning\nlatency,delay\n").unwrap();

        let text = MacBoundedUtf8FileReader
            .read_utf8(&path, 1024)
            .await
            .unwrap();

        assert!(text.contains("latency"));
    }

    #[tokio::test]
    async fn rejects_directory_and_oversized_file() {
        let dir = tempdir().unwrap();
        let directory_error = MacBoundedUtf8FileReader
            .read_utf8(dir.path(), 1024)
            .await
            .unwrap_err();
        assert_eq!(directory_error, BoundedUtf8FileReadError::InvalidFile);

        let path = dir.path().join("large.csv");
        fs::write(&path, [b'x'; 32]).unwrap();
        let size_error = MacBoundedUtf8FileReader
            .read_utf8(&path, 16)
            .await
            .unwrap_err();
        assert_eq!(size_error, BoundedUtf8FileReadError::TooLarge);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symlink_without_disclosing_its_path() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let target = dir.path().join("target.csv");
        let link = dir.path().join("words-link.csv");
        fs::write(&target, "term,meaning\nlatency,delay\n").unwrap();
        symlink(&target, &link).unwrap();

        let error = MacBoundedUtf8FileReader
            .read_utf8(&link, 1024)
            .await
            .unwrap_err();

        assert_eq!(error, BoundedUtf8FileReadError::InvalidFile);
        assert!(!error.to_string().contains(link.to_string_lossy().as_ref()));
    }
}
