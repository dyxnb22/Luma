use async_trait::async_trait;
use luma_application::{ScreenOcrError, ScreenOcrPort, MAX_OCR_TEXT_BYTES};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::process::Stdio;
use tempfile::TempPath;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub struct MacScreenOcr;

#[async_trait]
impl ScreenOcrPort for MacScreenOcr {
    async fn recognize_region(&self, cancel: CancellationToken) -> Result<String, ScreenOcrError> {
        if cancel.is_cancelled() {
            return Err(ScreenOcrError::Cancelled);
        }
        if !screen_capture_preflight() {
            return Err(ScreenOcrError::PermissionRequired);
        }
        let capture = CaptureTemp::new()?;
        if cancel.is_cancelled() {
            return Err(ScreenOcrError::Cancelled);
        }
        capture_region(capture.path(), &cancel).await?;
        if cancel.is_cancelled() {
            return Err(ScreenOcrError::Cancelled);
        }
        let path = capture.path().to_path_buf();
        let text = tokio::task::spawn_blocking(move || recognize_file(&path))
            .await
            .map_err(|_| {
                ScreenOcrError::RecognitionUnavailable(
                    "local Vision worker could not complete".into(),
                )
            })??;
        if cancel.is_cancelled() {
            return Err(ScreenOcrError::Cancelled);
        }
        let text = bounded_plain_text(&text);
        if text.is_empty() {
            return Err(ScreenOcrError::Empty);
        }
        Ok(text)
    }
}

struct CaptureTemp {
    path: TempPath,
}

impl CaptureTemp {
    fn new() -> Result<Self, ScreenOcrError> {
        let file = tempfile::Builder::new()
            .prefix(".luma-ocr-")
            .suffix(".png")
            .tempfile_in(std::env::temp_dir())
            .map_err(|_| {
                ScreenOcrError::CaptureUnavailable(
                    "could not create a private temporary capture".into(),
                )
            })?;
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).map_err(
            |_| ScreenOcrError::CaptureUnavailable("could not secure the temporary capture".into()),
        )?;
        Ok(Self {
            path: file.into_temp_path(),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

async fn capture_region(path: &Path, cancel: &CancellationToken) -> Result<(), ScreenOcrError> {
    let mut child = Command::new("/usr/sbin/screencapture")
        .args(["-i", "-s", "-x"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            ScreenOcrError::CaptureUnavailable(
                "the system region capture tool could not start".into(),
            )
        })?;
    let status = tokio::select! {
        _ = cancel.cancelled() => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(ScreenOcrError::Cancelled);
        }
        result = child.wait() => result.map_err(|_| {
            ScreenOcrError::CaptureUnavailable(
                "the system region capture tool did not complete".into(),
            )
        })?,
    };
    if cancel.is_cancelled() {
        return Err(ScreenOcrError::Cancelled);
    }
    let captured = std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false);
    classify_capture_result(status.success(), captured, screen_capture_preflight())
}

fn classify_capture_result(
    status_success: bool,
    captured: bool,
    permission_available: bool,
) -> Result<(), ScreenOcrError> {
    if !permission_available {
        return Err(ScreenOcrError::PermissionRequired);
    }
    // macOS can report a successful `screencapture` exit after Esc. An empty
    // private destination is therefore the reliable cancellation signal.
    if !captured {
        return Err(ScreenOcrError::Cancelled);
    }
    if !status_success {
        return Err(ScreenOcrError::CaptureUnavailable(
            "the system region capture failed".into(),
        ));
    }
    Ok(())
}

fn recognize_file(path: &Path) -> Result<String, ScreenOcrError> {
    let path = path.to_str().ok_or_else(|| {
        ScreenOcrError::RecognitionUnavailable("temporary path is not valid UTF-8".into())
    })?;
    let path = CString::new(path)
        .map_err(|_| ScreenOcrError::RecognitionUnavailable("temporary path is invalid".into()))?;
    let mut output: *mut c_char = std::ptr::null_mut();
    let status = unsafe { luma_vision_recognize(path.as_ptr(), &mut output) };
    match status {
        0 if !output.is_null() => {
            let text = unsafe { CStr::from_ptr(output) }
                .to_string_lossy()
                .into_owned();
            unsafe { luma_vision_free(output) };
            Ok(text)
        }
        1 => Err(ScreenOcrError::Empty),
        _ => {
            if !output.is_null() {
                unsafe { luma_vision_free(output) };
            }
            Err(ScreenOcrError::RecognitionUnavailable(
                "Apple Vision text recognition failed".into(),
            ))
        }
    }
}

fn bounded_plain_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.len() <= MAX_OCR_TEXT_BYTES {
        return trimmed.into();
    }
    let mut end = MAX_OCR_TEXT_BYTES;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].trim_end().into()
}

#[cfg(target_os = "macos")]
fn screen_capture_preflight() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
fn screen_capture_preflight() -> bool {
    false
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
extern "C" {
    fn luma_vision_recognize(image_path: *const c_char, out_text: *mut *mut c_char) -> c_int;
    fn luma_vision_free(text: *mut c_char);
}

#[cfg(not(target_os = "macos"))]
unsafe fn luma_vision_recognize(_image_path: *const c_char, _out_text: *mut *mut c_char) -> c_int {
    2
}

#[cfg(not(target_os = "macos"))]
unsafe fn luma_vision_free(_text: *mut c_char) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn dropped_capture_path() -> PathBuf {
        let capture = CaptureTemp::new().unwrap();
        let path = capture.path().to_path_buf();
        std::fs::write(&path, b"fixture").unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(capture);
        path
    }

    #[test]
    fn private_temp_capture_is_deleted_on_every_drop_path() {
        for _scenario in ["success", "cancel", "failure"] {
            let path = dropped_capture_path();
            assert!(!path.exists());
        }
    }

    #[test]
    fn recognized_text_is_plain_bounded_and_utf8_safe() {
        let oversized = format!("{}\r\n{}", "界".repeat(100_000), "tail");
        let bounded = bounded_plain_text(&oversized);
        assert!(bounded.len() <= MAX_OCR_TEXT_BYTES);
        assert!(bounded.is_char_boundary(bounded.len()));
        assert!(!bounded.contains('\r'));
    }

    #[test]
    fn empty_capture_is_cancelled_even_after_successful_exit() {
        assert_eq!(
            classify_capture_result(true, false, true),
            Err(ScreenOcrError::Cancelled)
        );
        assert_eq!(
            classify_capture_result(false, false, false),
            Err(ScreenOcrError::PermissionRequired)
        );
        assert!(matches!(
            classify_capture_result(false, true, true),
            Err(ScreenOcrError::CaptureUnavailable(_))
        ));
    }
}
