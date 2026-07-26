use async_trait::async_trait;
use std::cmp::Reverse;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_DOWNLOAD_ENTRIES: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadCategory {
    Archive,
    Image,
    Video,
    Document,
    Installer,
    Other,
}

impl DownloadCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Archive => "archive",
            Self::Image => "image",
            Self::Video => "video",
            Self::Document => "document",
            Self::Installer => "installer",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "archive" => Self::Archive,
            "image" => Self::Image,
            "video" => Self::Video,
            "document" => Self::Document,
            "installer" => Self::Installer,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadsFilter {
    Recent,
    Large,
    Old { days: u32 },
    Type(DownloadCategory),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadEntry {
    /// Stable path-derived identifier. It does not expose the path.
    pub id: String,
    /// Fresh filesystem identity used to reject replacement races.
    pub identity: String,
    pub display_name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_unix: i64,
    pub category: DownloadCategory,
    pub is_directory: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DownloadsError {
    #[error("Downloads folder is not configured")]
    NotConfigured,
    #[error("Downloads unavailable: {0}")]
    Unavailable(String),
    #[error("download item not found")]
    NotFound,
    #[error("download item changed since it was listed")]
    StaleIdentity,
    #[error("invalid download operation: {0}")]
    Invalid(String),
    #[error("download destination already exists")]
    Conflict,
    #[error("download operation cancelled")]
    Cancelled,
    #[error("download I/O failed: {0}")]
    Io(String),
}

#[async_trait]
pub trait DownloadsPort: Send + Sync {
    async fn list(
        &self,
        filter: DownloadsFilter,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<DownloadEntry>, DownloadsError>;

    async fn resolve(
        &self,
        id: &str,
        cancel: CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError>;

    async fn rename(
        &self,
        id: &str,
        expected_identity: &str,
        new_name: &str,
        cancel: CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError>;

    async fn trash(
        &self,
        id: &str,
        expected_identity: &str,
        cancel: CancellationToken,
    ) -> Result<(), DownloadsError>;
}

/// In-memory fake for module tests. It never reads Downloads or invokes Finder.
#[derive(Default)]
pub struct FakeDownloads {
    entries: Mutex<Vec<DownloadEntry>>,
    pub rename_calls: Arc<Mutex<Vec<(String, String)>>>,
    pub trash_calls: Arc<Mutex<Vec<String>>>,
    error: Mutex<Option<DownloadsError>>,
}

impl FakeDownloads {
    pub fn new(entries: Vec<DownloadEntry>) -> Self {
        Self {
            entries: Mutex::new(entries),
            ..Self::default()
        }
    }

    pub fn fail_with(&self, error: DownloadsError) {
        *self.error.lock().expect("downloads error lock") = Some(error);
    }

    pub fn replace(&self, entries: Vec<DownloadEntry>) {
        *self.entries.lock().expect("downloads entries lock") = entries;
    }

    fn take_error(&self) -> Result<(), DownloadsError> {
        match self.error.lock().expect("downloads error lock").take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl DownloadsPort for FakeDownloads {
    async fn list(
        &self,
        filter: DownloadsFilter,
        limit: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<DownloadEntry>, DownloadsError> {
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        self.take_error()?;
        let mut entries = self.entries.lock().expect("downloads entries lock").clone();
        match filter {
            DownloadsFilter::Recent => entries.sort_by_key(|entry| Reverse(entry.modified_unix)),
            DownloadsFilter::Large => {
                entries.retain(|entry| !entry.is_directory);
                entries.sort_by_key(|entry| Reverse(entry.size_bytes));
            }
            DownloadsFilter::Old { days } => {
                let cutoff = chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(i64::from(days) * 86_400);
                entries.retain(|entry| entry.modified_unix <= cutoff);
                entries.sort_by_key(|entry| entry.modified_unix);
            }
            DownloadsFilter::Type(category) => {
                entries.retain(|entry| entry.category == category);
                entries.sort_by_key(|entry| Reverse(entry.modified_unix));
            }
            DownloadsFilter::Text(needle) => {
                let needle = needle.to_lowercase();
                entries.retain(|entry| entry.display_name.to_lowercase().contains(&needle));
                entries.sort_by_key(|entry| Reverse(entry.modified_unix));
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
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        self.take_error()?;
        self.entries
            .lock()
            .expect("downloads entries lock")
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
            .ok_or(DownloadsError::NotFound)
    }

    async fn rename(
        &self,
        id: &str,
        expected_identity: &str,
        new_name: &str,
        cancel: CancellationToken,
    ) -> Result<DownloadEntry, DownloadsError> {
        if cancel.is_cancelled() {
            return Err(DownloadsError::Cancelled);
        }
        self.take_error()?;
        let mut entries = self.entries.lock().expect("downloads entries lock");
        let entry = entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(DownloadsError::NotFound)?;
        if entry.identity != expected_identity {
            return Err(DownloadsError::StaleIdentity);
        }
        self.rename_calls
            .lock()
            .expect("downloads rename calls lock")
            .push((id.into(), new_name.into()));
        entry.display_name = new_name.into();
        entry.path.set_file_name(new_name);
        entry.id = format!("{id}:renamed");
        entry.identity = format!("{expected_identity}:renamed");
        Ok(entry.clone())
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
        self.take_error()?;
        let mut entries = self.entries.lock().expect("downloads entries lock");
        let index = entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(DownloadsError::NotFound)?;
        if entries[index].identity != expected_identity {
            return Err(DownloadsError::StaleIdentity);
        }
        self.trash_calls
            .lock()
            .expect("downloads trash calls lock")
            .push(id.into());
        entries.remove(index);
        Ok(())
    }
}
