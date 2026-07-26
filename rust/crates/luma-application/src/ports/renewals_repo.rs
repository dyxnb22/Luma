use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

pub const MAX_RENEWALS: usize = 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenewalEntry {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub cadence_kind: String,
    pub cadence_value: Option<i64>,
    pub anchor_month: Option<u32>,
    pub anchor_day: Option<u32>,
    pub next_due_date: String,
    pub auto_renew: bool,
    pub status: String,
    pub url: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRenewal {
    pub name: String,
    pub category: String,
    pub amount_minor: Option<i64>,
    pub currency: Option<String>,
    pub cadence_kind: String,
    pub cadence_value: Option<i64>,
    pub anchor_month: Option<u32>,
    pub anchor_day: Option<u32>,
    pub next_due_date: String,
    pub auto_renew: bool,
    pub status: String,
    pub url: Option<String>,
    pub note: Option<String>,
    pub now: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenewalPaidUpdate {
    pub id: i64,
    pub expected_due_date: String,
    pub expected_updated_at: String,
    pub next_due_date: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RenewalsRepoError {
    #[error("renewal not found")]
    NotFound,
    #[error("renewal changed since it was shown")]
    Conflict,
    #[error("renewals capacity reached ({MAX_RENEWALS})")]
    Capacity,
    #[error("renewals store: {0}")]
    Store(String),
}

pub trait RenewalsRepository: Send + Sync {
    fn list(&self) -> Result<Vec<RenewalEntry>, RenewalsRepoError>;
    fn get(&self, id: i64) -> Result<Option<RenewalEntry>, RenewalsRepoError>;
    fn insert(&self, entry: &NewRenewal) -> Result<RenewalEntry, RenewalsRepoError>;
    fn update(
        &self,
        entry: &RenewalEntry,
        expected_updated_at: &str,
    ) -> Result<(), RenewalsRepoError>;
    fn mark_paid(&self, update: &RenewalPaidUpdate) -> Result<(), RenewalsRepoError>;
    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), RenewalsRepoError>;
    fn backup(&self) -> Result<PathBuf, RenewalsRepoError>;
}

pub struct MemoryRenewalsRepository {
    entries: Mutex<Vec<RenewalEntry>>,
    next_id: Mutex<i64>,
}

impl Default for MemoryRenewalsRepository {
    fn default() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl RenewalsRepository for MemoryRenewalsRepository {
    fn list(&self) -> Result<Vec<RenewalEntry>, RenewalsRepoError> {
        let mut entries = self.entries.lock().expect("renewals memory lock").clone();
        entries.sort_by(|a, b| {
            a.next_due_date
                .cmp(&b.next_due_date)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(entries)
    }

    fn get(&self, id: i64) -> Result<Option<RenewalEntry>, RenewalsRepoError> {
        Ok(self
            .entries
            .lock()
            .expect("renewals memory lock")
            .iter()
            .find(|entry| entry.id == id)
            .cloned())
    }

    fn insert(&self, entry: &NewRenewal) -> Result<RenewalEntry, RenewalsRepoError> {
        let mut entries = self.entries.lock().expect("renewals memory lock");
        if entries.len() >= MAX_RENEWALS {
            return Err(RenewalsRepoError::Capacity);
        }
        let mut next_id = self.next_id.lock().expect("renewals id lock");
        let stored = RenewalEntry {
            id: *next_id,
            name: entry.name.clone(),
            category: entry.category.clone(),
            amount_minor: entry.amount_minor,
            currency: entry.currency.clone(),
            cadence_kind: entry.cadence_kind.clone(),
            cadence_value: entry.cadence_value,
            anchor_month: entry.anchor_month,
            anchor_day: entry.anchor_day,
            next_due_date: entry.next_due_date.clone(),
            auto_renew: entry.auto_renew,
            status: entry.status.clone(),
            url: entry.url.clone(),
            note: entry.note.clone(),
            created_at: entry.now.clone(),
            updated_at: entry.now.clone(),
        };
        *next_id += 1;
        entries.push(stored.clone());
        Ok(stored)
    }

    fn update(
        &self,
        entry: &RenewalEntry,
        expected_updated_at: &str,
    ) -> Result<(), RenewalsRepoError> {
        let mut entries = self.entries.lock().expect("renewals memory lock");
        let current = entries
            .iter_mut()
            .find(|current| current.id == entry.id)
            .ok_or(RenewalsRepoError::NotFound)?;
        if current.updated_at != expected_updated_at {
            return Err(RenewalsRepoError::Conflict);
        }
        *current = entry.clone();
        Ok(())
    }

    fn mark_paid(&self, update: &RenewalPaidUpdate) -> Result<(), RenewalsRepoError> {
        let mut entries = self.entries.lock().expect("renewals memory lock");
        let current = entries
            .iter_mut()
            .find(|current| current.id == update.id)
            .ok_or(RenewalsRepoError::NotFound)?;
        if current.updated_at != update.expected_updated_at
            || current.next_due_date != update.expected_due_date
        {
            return Err(RenewalsRepoError::Conflict);
        }
        current.next_due_date = update.next_due_date.clone();
        current.status = update.status.clone();
        current.updated_at = update.updated_at.clone();
        Ok(())
    }

    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), RenewalsRepoError> {
        let mut entries = self.entries.lock().expect("renewals memory lock");
        let index = entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(RenewalsRepoError::NotFound)?;
        if entries[index].updated_at != expected_updated_at {
            return Err(RenewalsRepoError::Conflict);
        }
        entries.remove(index);
        Ok(())
    }

    fn backup(&self) -> Result<PathBuf, RenewalsRepoError> {
        Ok(PathBuf::from("/fixture/renewals-backup.sqlite"))
    }
}
