//! Keychain port — labels in search; values only via explicit copy.
//! Uses the `security` CLI against a disposable service namespace.

use async_trait::async_trait;
use luma_storage::luma_next_support_dir;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

pub use luma_application::{FakeKeychain, KeychainError, KeychainPort as Keychain, SecretLabel};

const DEFAULT_SERVICE: &str = "com.luma.next.secrets";

pub struct MacKeychain {
    service: String,
}

#[derive(Default, Serialize, Deserialize)]
struct LabelSidecar {
    labels: Vec<String>,
}

impl MacKeychain {
    pub fn luma_next() -> Self {
        Self {
            service: DEFAULT_SERVICE.into(),
        }
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn labels_path() -> Result<std::path::PathBuf, KeychainError> {
        let path = luma_next_support_dir()
            .map_err(|err| KeychainError::Unavailable(err.to_string()))?
            .join("secrets-labels.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(path)
    }

    fn read_labels() -> Result<LabelSidecar, KeychainError> {
        let path = Self::labels_path()?;
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|err| KeychainError::Unavailable(format!("invalid label sidecar: {err}"))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(LabelSidecar::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn write_labels(labels: &LabelSidecar) -> Result<(), KeychainError> {
        let path = Self::labels_path()?;
        let bytes = serde_json::to_vec_pretty(labels)
            .map_err(|err| KeychainError::Unavailable(err.to_string()))?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

#[async_trait]
impl Keychain for MacKeychain {
    async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError> {
        // Never invoke `security dump-keychain`: it can expose values and prompt.
        Ok(Self::read_labels()?
            .labels
            .into_iter()
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
        let status = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                &self.service,
                "-a",
                account,
                "-w",
                password,
                "-U",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await?;
        if status.success() {
            let mut labels = Self::read_labels()?;
            if !labels.labels.iter().any(|label| label == account) {
                labels.labels.push(account.to_owned());
                labels.labels.sort();
                Self::write_labels(&labels)?;
            }
            Ok(())
        } else {
            Err(KeychainError::Unavailable(format!(
                "security add-generic-password exited {status}"
            )))
        }
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
            let mut labels = Self::read_labels()?;
            labels.labels.retain(|label| label != account);
            Self::write_labels(&labels)?;
            Ok(())
        } else {
            Err(KeychainError::NotFound(account.into()))
        }
    }
}
