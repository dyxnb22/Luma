//! Reminders / EventKit capability port.
//!
//! Full `EKEventStore` needs an app bundle + Info.plist usage string. The live
//! adapter only probes what a bare CLI can do and returns structured status —
//! it never pretends auth succeeded.

use async_trait::async_trait;

pub use luma_application::{EventKitError, EventKitPort as EventKit, ReminderItem, RemindersAuth};

/// Live probe: bare CLI cannot prompt EventKit.
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
