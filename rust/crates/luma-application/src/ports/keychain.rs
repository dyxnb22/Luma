use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain unavailable: {0}")]
    Unavailable(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct SecretLabel {
    pub account: String,
}

#[async_trait]
pub trait KeychainPort: Send + Sync {
    async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError>;
    async fn copy_password(&self, account: &str) -> Result<String, KeychainError>;
    async fn set_password(&self, account: &str, password: &str) -> Result<(), KeychainError>;
    async fn delete(&self, account: &str) -> Result<(), KeychainError>;
}

pub struct FakeKeychain {
    pub unlocked: bool,
    pub entries: tokio::sync::Mutex<std::collections::BTreeMap<String, String>>,
}

#[async_trait]
impl KeychainPort for FakeKeychain {
    async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError> {
        if !self.unlocked {
            return Ok(Vec::new());
        }
        Ok(self
            .entries
            .lock()
            .await
            .keys()
            .map(|account| SecretLabel {
                account: account.clone(),
            })
            .collect())
    }

    async fn copy_password(&self, account: &str) -> Result<String, KeychainError> {
        if !self.unlocked {
            return Err(KeychainError::Unavailable("locked".into()));
        }
        self.entries
            .lock()
            .await
            .get(account)
            .cloned()
            .ok_or_else(|| KeychainError::NotFound(account.into()))
    }

    async fn set_password(&self, account: &str, password: &str) -> Result<(), KeychainError> {
        self.entries
            .lock()
            .await
            .insert(account.into(), password.into());
        Ok(())
    }

    async fn delete(&self, account: &str) -> Result<(), KeychainError> {
        if self.entries.lock().await.remove(account).is_some() {
            Ok(())
        } else {
            Err(KeychainError::NotFound(account.into()))
        }
    }
}
