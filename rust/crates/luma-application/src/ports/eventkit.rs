use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemindersAuth {
    NotDetermined,
    Denied,
    Authorized,
}

#[derive(Debug, Error)]
pub enum EventKitError {
    #[error("reminders not authorized")]
    NotAuthorized,
    #[error("eventkit unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug)]
pub struct ReminderItem {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

#[async_trait]
pub trait EventKitPort: Send + Sync {
    async fn auth_status(&self) -> RemindersAuth;
    async fn request_access(&self) -> Result<RemindersAuth, EventKitError>;
    async fn list_incomplete(&self) -> Result<Vec<ReminderItem>, EventKitError>;
    async fn create(&self, title: &str) -> Result<ReminderItem, EventKitError>;
    async fn complete(&self, id: &str) -> Result<(), EventKitError>;
}
