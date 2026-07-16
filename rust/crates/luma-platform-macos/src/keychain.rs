//! Keychain port — labels in search; values only via explicit copy.
//!
//! Read/delete operations use the macOS `security` CLI without passing values. Password writes
//! use the Security framework directly so a secret never appears in a child-process argument
//! list, environment, log, or error message.

use async_trait::async_trait;
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::process::Command;

pub use luma_application::{FakeKeychain, KeychainError, KeychainPort as Keychain, SecretLabel};

const DEFAULT_SERVICE: &str = "com.luma.next.secrets";
const PRIVATE_REFERENCE_PREFIX: &str = "proxy-profile-url:";
static LABEL_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct MacKeychain {
    service: String,
    writer: Arc<dyn PasswordWriter>,
    manages_labels: bool,
    #[cfg(test)]
    labels_path: Option<std::path::PathBuf>,
}

/// Platform-local write boundary. Keeping it separate lets tests verify the Keychain port with
/// an in-memory writer instead of touching a user's Keychain.
trait PasswordWriter: Send + Sync {
    fn upsert(&self, service: &str, account: &str, password: &[u8]) -> Result<(), KeychainError>;
}

struct SecurityFrameworkWriter;

#[cfg(target_os = "macos")]
impl PasswordWriter for SecurityFrameworkWriter {
    fn upsert(&self, service: &str, account: &str, password: &[u8]) -> Result<(), KeychainError> {
        native_upsert_password(service, account, password)
    }
}

#[cfg(not(target_os = "macos"))]
impl PasswordWriter for SecurityFrameworkWriter {
    fn upsert(
        &self,
        _service: &str,
        _account: &str,
        _password: &[u8],
    ) -> Result<(), KeychainError> {
        Err(KeychainError::Unavailable(
            "macOS Keychain is unavailable on this platform".into(),
        ))
    }
}

#[derive(Default, Serialize, Deserialize)]
struct LabelSidecar {
    labels: Vec<String>,
}

impl MacKeychain {
    pub fn luma_next() -> Self {
        Self {
            service: DEFAULT_SERVICE.into(),
            writer: Arc::new(SecurityFrameworkWriter),
            manages_labels: true,
            #[cfg(test)]
            labels_path: None,
        }
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            writer: Arc::new(SecurityFrameworkWriter),
            manages_labels: true,
            #[cfg(test)]
            labels_path: None,
        }
    }

    /// Uses the same Keychain service but never publishes account names to the Secrets label
    /// sidecar. This is for internal references such as Profile subscription URLs, which must
    /// not become copyable Secret-module entries.
    pub fn private_references() -> Self {
        Self {
            service: DEFAULT_SERVICE.into(),
            writer: Arc::new(SecurityFrameworkWriter),
            manages_labels: false,
            #[cfg(test)]
            labels_path: None,
        }
    }

    #[cfg(test)]
    fn with_writer(
        service: impl Into<String>,
        labels_path: std::path::PathBuf,
        writer: Arc<dyn PasswordWriter>,
        manages_labels: bool,
    ) -> Self {
        Self {
            service: service.into(),
            writer,
            manages_labels,
            labels_path: Some(labels_path),
        }
    }

    fn labels_path(&self) -> Result<std::path::PathBuf, KeychainError> {
        #[cfg(test)]
        if let Some(path) = &self.labels_path {
            return Ok(path.clone());
        }
        let path = luma_next_support_dir()
            .map_err(|err| KeychainError::Unavailable(err.to_string()))?
            .join("secrets-labels.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(path)
    }

    fn read_labels(&self) -> Result<LabelSidecar, KeychainError> {
        let path = self.labels_path()?;
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|err| KeychainError::Unavailable(format!("invalid label sidecar: {err}"))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(LabelSidecar::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn write_labels(&self, labels: &LabelSidecar) -> Result<(), KeychainError> {
        let path = self.labels_path()?;
        let bytes = serde_json::to_vec_pretty(labels)
            .map_err(|err| KeychainError::Unavailable(err.to_string()))?;
        write_labels_atomically(&path, &bytes)
    }
}

/// Labels are private local metadata. Retain the old file on any write failure and commit the
/// replacement with a same-directory rename so a crash cannot leave a truncated label list.
fn write_labels_atomically(path: &Path, bytes: &[u8]) -> Result<(), KeychainError> {
    let parent = path
        .parent()
        .ok_or_else(|| KeychainError::Unavailable("secret label metadata is unavailable".into()))?;
    fs::create_dir_all(parent)
        .map_err(|_| KeychainError::Unavailable("secret label metadata is unavailable".into()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| KeychainError::Unavailable("secret label metadata is unavailable".into()))?;
    for _ in 0..16 {
        let sequence = LABEL_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temporary) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(KeychainError::Unavailable(
                    "secret label metadata is unavailable".into(),
                ));
            }
        };
        let result = file.write_all(bytes).and_then(|_| file.sync_all());
        drop(file);
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(KeychainError::Unavailable(
                "secret label metadata is unavailable".into(),
            ));
        }
        if fs::rename(&temporary, path).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(KeychainError::Unavailable(
                "secret label metadata is unavailable".into(),
            ));
        }
        let _ = fs::File::open(parent).and_then(|directory| directory.sync_all());
        return Ok(());
    }
    Err(KeychainError::Unavailable(
        "secret label metadata is unavailable".into(),
    ))
}

#[async_trait]
impl Keychain for MacKeychain {
    async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError> {
        // Never invoke `security dump-keychain`: it can expose values and prompt.
        if !self.manages_labels {
            return Ok(Vec::new());
        }
        Ok(self
            .read_labels()?
            .labels
            .into_iter()
            // Older builds incorrectly put internal Profile URL references in this shared
            // sidecar. Hide the reserved namespace immediately without reading their values.
            .filter(|account| !account.starts_with(PRIVATE_REFERENCE_PREFIX))
            .map(|account| SecretLabel { account })
            .collect())
    }

    async fn copy_password(&self, account: &str) -> Result<String, KeychainError> {
        let out = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                &self.service,
                "-a",
                account,
                "-w",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !out.status.success() {
            return Err(KeychainError::NotFound(account.into()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    async fn set_password(&self, account: &str, password: &str) -> Result<(), KeychainError> {
        let service = self.service.clone();
        let account = account.to_owned();
        let writer_account = account.clone();
        let password = password.as_bytes().to_vec();
        let writer = Arc::clone(&self.writer);
        tokio::task::spawn_blocking(move || writer.upsert(&service, &writer_account, &password))
            .await
            .map_err(|_| KeychainError::Unavailable("Keychain write task failed".into()))??;

        if self.manages_labels {
            let mut labels = self.read_labels()?;
            if !labels.labels.iter().any(|label| label == &account) {
                labels.labels.push(account);
                labels.labels.sort();
                self.write_labels(&labels)?;
            }
        }
        Ok(())
    }

    async fn delete(&self, account: &str) -> Result<(), KeychainError> {
        let status = Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                &self.service,
                "-a",
                account,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
        if status.success() {
            if self.manages_labels {
                let mut labels = self.read_labels()?;
                labels.labels.retain(|label| label != account);
                self.write_labels(&labels)?;
            }
            Ok(())
        } else {
            Err(KeychainError::NotFound(account.into()))
        }
    }
}

#[cfg(target_os = "macos")]
fn native_upsert_password(
    service: &str,
    account: &str,
    password: &[u8],
) -> Result<(), KeychainError> {
    use std::ffi::c_void;

    type SecKeychainItemRef = *mut c_void;
    const ERR_SEC_SUCCESS: i32 = 0;
    const ERR_SEC_DUPLICATE_ITEM: i32 = -25_299;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecKeychainAddGenericPassword(
            keychain: *mut c_void,
            service_name_length: u32,
            service_name: *const std::ffi::c_char,
            account_name_length: u32,
            account_name: *const std::ffi::c_char,
            password_length: u32,
            password_data: *const c_void,
            item_ref: *mut SecKeychainItemRef,
        ) -> i32;
        fn SecKeychainFindGenericPassword(
            keychain_or_array: *mut c_void,
            service_name_length: u32,
            service_name: *const std::ffi::c_char,
            account_name_length: u32,
            account_name: *const std::ffi::c_char,
            password_length: *mut u32,
            password_data: *mut *mut c_void,
            item_ref: *mut SecKeychainItemRef,
        ) -> i32;
        fn SecKeychainItemModifyAttributesAndData(
            item_ref: SecKeychainItemRef,
            attributes: *const c_void,
            password_length: u32,
            password_data: *const c_void,
        ) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    let service_len = u32::try_from(service.len())
        .map_err(|_| KeychainError::Unavailable("Keychain service name is too long".into()))?;
    let account_len = u32::try_from(account.len())
        .map_err(|_| KeychainError::Unavailable("Keychain account name is too long".into()))?;
    let password_len = u32::try_from(password.len())
        .map_err(|_| KeychainError::Unavailable("Keychain value is too long".into()))?;

    // SAFETY: the byte slices remain live for the duration of each synchronous Security.framework
    // call. Their explicit lengths let Keychain accept arbitrary UTF-8 password bytes without a
    // NUL terminator, and no returned item reference is requested on insertion.
    let status = unsafe {
        SecKeychainAddGenericPassword(
            std::ptr::null_mut(),
            service_len,
            service.as_ptr().cast(),
            account_len,
            account.as_ptr().cast(),
            password_len,
            password.as_ptr().cast(),
            std::ptr::null_mut(),
        )
    };
    if status == ERR_SEC_SUCCESS {
        return Ok(());
    }
    if status != ERR_SEC_DUPLICATE_ITEM {
        return Err(KeychainError::Unavailable(
            "Keychain could not store the password".into(),
        ));
    }

    let mut item: SecKeychainItemRef = std::ptr::null_mut();
    // SAFETY: service/account buffers remain live and we request only an owned item reference;
    // password output pointers are null so no Keychain-owned password buffer is allocated.
    let find_status = unsafe {
        SecKeychainFindGenericPassword(
            std::ptr::null_mut(),
            service_len,
            service.as_ptr().cast(),
            account_len,
            account.as_ptr().cast(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut item,
        )
    };
    if find_status != ERR_SEC_SUCCESS || item.is_null() {
        return Err(KeychainError::Unavailable(
            "Keychain could not update the password".into(),
        ));
    }
    // SAFETY: `item` is a valid owned reference returned above and all password bytes remain
    // alive for the synchronous update. CFRelease balances the returned ownership exactly once.
    let update_status = unsafe {
        let status = SecKeychainItemModifyAttributesAndData(
            item,
            std::ptr::null(),
            password_len,
            password.as_ptr().cast(),
        );
        CFRelease(item.cast_const());
        status
    };
    if update_status == ERR_SEC_SUCCESS {
        Ok(())
    } else {
        Err(KeychainError::Unavailable(
            "Keychain could not update the password".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingWriter {
        calls: Mutex<Vec<(String, String, Vec<u8>)>>,
    }

    impl PasswordWriter for RecordingWriter {
        fn upsert(
            &self,
            service: &str,
            account: &str,
            password: &[u8],
        ) -> Result<(), KeychainError> {
            self.calls.lock().expect("recording writer lock").push((
                service.to_string(),
                account.to_string(),
                password.to_vec(),
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn set_password_uses_in_memory_writer_and_sidecar_has_no_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let writer = Arc::new(RecordingWriter::default());
        let keychain = MacKeychain::with_writer(
            "com.luma.test",
            dir.path().join("labels.json"),
            writer.clone(),
            true,
        );
        let value = format!("test-{}", "value");

        keychain
            .set_password("test-account", &value)
            .await
            .expect("fake keychain write");

        assert_eq!(
            writer
                .calls
                .lock()
                .expect("recording writer lock")
                .as_slice(),
            [(
                "com.luma.test".into(),
                "test-account".into(),
                value.as_bytes().to_vec()
            )]
        );
        let sidecar =
            std::fs::read_to_string(dir.path().join("labels.json")).expect("labels sidecar");
        assert!(sidecar.contains("test-account"));
        assert!(!sidecar.contains(&value));
    }

    #[tokio::test]
    async fn private_references_never_become_secret_labels() {
        let dir = tempfile::tempdir().expect("tempdir");
        let labels_path = dir.path().join("labels.json");
        let writer = Arc::new(RecordingWriter::default());
        let private =
            MacKeychain::with_writer("com.luma.test", labels_path.clone(), writer.clone(), false);

        private
            .set_password("proxy-profile-url:p-0123456789abcdef0123", "test-url")
            .await
            .expect("private fake keychain write");
        assert!(!labels_path.exists());

        std::fs::write(
            &labels_path,
            r#"{"labels":["visible","proxy-profile-url:p-legacy"]}"#,
        )
        .expect("legacy labels");
        let managed = MacKeychain::with_writer("com.luma.test", labels_path, writer, true);
        let labels = managed.list_labels().await.expect("labels");
        assert_eq!(
            labels
                .into_iter()
                .map(|label| label.account)
                .collect::<Vec<_>>(),
            ["visible"]
        );
    }
}
