//! Reminders / EventKit capability port.
//! Full EKEventStore requires an app bundle + Info.plist usage string.
//! This adapter probes what a bare CLI can do and returns structured status.

use async_trait::async_trait;

pub use luma_application::{EventKitError, EventKitPort as EventKit, ReminderItem, RemindersAuth};

/// Live probe: bare CLI cannot prompt EventKit; reports NotDetermined unless overridden.
pub struct MacEventKit;

#[async_trait]
impl EventKit for MacEventKit {
    async fn auth_status(&self) -> RemindersAuth {
        RemindersAuth::NotDetermined
    }

    async fn request_access(&self) -> Result<RemindersAuth, EventKitError> {
        Err(EventKitError::Unavailable(
            "EventKit auth prompt requires a signed app bundle with NSRemindersUsageDescription; CLI probe cannot request".into(),
        ))
    }

    async fn list_incomplete(&self) -> Result<Vec<ReminderItem>, EventKitError> {
        Err(EventKitError::NotAuthorized)
    }

    async fn create(&self, _title: &str) -> Result<ReminderItem, EventKitError> {
        Err(EventKitError::NotAuthorized)
    }

    async fn complete(&self, _id: &str) -> Result<(), EventKitError> {
        Err(EventKitError::NotAuthorized)
    }
}

pub struct FakeEventKit {
    pub status: RemindersAuth,
    pub items: tokio::sync::Mutex<Vec<ReminderItem>>,
}

#[async_trait]
impl EventKit for FakeEventKit {
    async fn auth_status(&self) -> RemindersAuth {
        self.status
    }

    async fn request_access(&self) -> Result<RemindersAuth, EventKitError> {
        Ok(self.status)
    }

    async fn list_incomplete(&self) -> Result<Vec<ReminderItem>, EventKitError> {
        if self.status != RemindersAuth::Authorized {
            return Err(EventKitError::NotAuthorized);
        }
        Ok(self
            .items
            .lock()
            .await
            .iter()
            .filter(|i| !i.completed)
            .cloned()
            .collect())
    }

    async fn create(&self, title: &str) -> Result<ReminderItem, EventKitError> {
        if self.status != RemindersAuth::Authorized {
            return Err(EventKitError::NotAuthorized);
        }
        let item = ReminderItem {
            id: format!("fake:{title}"),
            title: title.to_string(),
            completed: false,
        };
        self.items.lock().await.push(item.clone());
        Ok(item)
    }

    async fn complete(&self, id: &str) -> Result<(), EventKitError> {
        if self.status != RemindersAuth::Authorized {
            return Err(EventKitError::NotAuthorized);
        }
        let mut items = self.items.lock().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.completed = true;
            Ok(())
        } else {
            Err(EventKitError::Unavailable(format!("missing {id}")))
        }
    }
}
