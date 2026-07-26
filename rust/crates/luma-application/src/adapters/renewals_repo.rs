use crate::ports::{
    NewRenewal, RenewalEntry, RenewalPaidUpdate, RenewalsRepoError, RenewalsRepository,
};
use luma_storage::{RenewalRow, RenewalsStore, RenewalsStoreError};
use std::path::PathBuf;
use std::sync::Arc;

pub struct SqliteRenewalsRepository {
    store: Arc<RenewalsStore>,
}

impl SqliteRenewalsRepository {
    pub fn new(store: Arc<RenewalsStore>) -> Self {
        Self { store }
    }
}

impl RenewalsRepository for SqliteRenewalsRepository {
    fn list(&self) -> Result<Vec<RenewalEntry>, RenewalsRepoError> {
        self.store
            .list()
            .map(|rows| rows.into_iter().map(entry_from_row).collect())
            .map_err(map_error)
    }

    fn get(&self, id: i64) -> Result<Option<RenewalEntry>, RenewalsRepoError> {
        self.store
            .get(id)
            .map(|row| row.map(entry_from_row))
            .map_err(map_error)
    }

    fn insert(&self, entry: &NewRenewal) -> Result<RenewalEntry, RenewalsRepoError> {
        self.store
            .insert(&RenewalRow {
                id: 0,
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
            })
            .map(entry_from_row)
            .map_err(map_error)
    }

    fn update(
        &self,
        entry: &RenewalEntry,
        expected_updated_at: &str,
    ) -> Result<(), RenewalsRepoError> {
        self.store
            .update(&row_from_entry(entry), expected_updated_at)
            .map_err(map_error)
    }

    fn mark_paid(&self, update: &RenewalPaidUpdate) -> Result<(), RenewalsRepoError> {
        self.store
            .mark_paid(
                update.id,
                &update.expected_due_date,
                &update.expected_updated_at,
                &update.next_due_date,
                &update.status,
                &update.updated_at,
            )
            .map_err(map_error)
    }

    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), RenewalsRepoError> {
        self.store
            .delete(id, expected_updated_at)
            .map_err(map_error)
    }

    fn backup(&self) -> Result<PathBuf, RenewalsRepoError> {
        self.store.backup().map_err(map_error)
    }
}

fn entry_from_row(row: RenewalRow) -> RenewalEntry {
    RenewalEntry {
        id: row.id,
        name: row.name,
        category: row.category,
        amount_minor: row.amount_minor,
        currency: row.currency,
        cadence_kind: row.cadence_kind,
        cadence_value: row.cadence_value,
        anchor_month: row.anchor_month,
        anchor_day: row.anchor_day,
        next_due_date: row.next_due_date,
        auto_renew: row.auto_renew,
        status: row.status,
        url: row.url,
        note: row.note,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn row_from_entry(entry: &RenewalEntry) -> RenewalRow {
    RenewalRow {
        id: entry.id,
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
        created_at: entry.created_at.clone(),
        updated_at: entry.updated_at.clone(),
    }
}

fn map_error(error: RenewalsStoreError) -> RenewalsRepoError {
    match error {
        RenewalsStoreError::NotFound => RenewalsRepoError::NotFound,
        RenewalsStoreError::Conflict => RenewalsRepoError::Conflict,
        RenewalsStoreError::Capacity => RenewalsRepoError::Capacity,
        other => RenewalsRepoError::Store(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_adapter_preserves_capacity_and_conflict_errors() {
        let temp = tempfile::tempdir().unwrap();
        let store =
            Arc::new(RenewalsStore::with_path(temp.path().join("renewals.sqlite")).unwrap());
        let repo = SqliteRenewalsRepository::new(store);
        let stored = repo
            .insert(&NewRenewal {
                name: "Cloud".into(),
                category: "software".into(),
                amount_minor: Some(999),
                currency: Some("USD".into()),
                cadence_kind: "monthly".into(),
                cadence_value: None,
                anchor_month: Some(1),
                anchor_day: Some(31),
                next_due_date: "2026-01-31".into(),
                auto_renew: true,
                status: "active".into(),
                url: None,
                note: None,
                now: "v1".into(),
            })
            .unwrap();
        let mut changed = stored.clone();
        changed.name = "Changed".into();
        changed.updated_at = "v2".into();
        repo.update(&changed, "v1").unwrap();
        assert_eq!(
            repo.update(&changed, "v1").unwrap_err(),
            RenewalsRepoError::Conflict
        );
    }
}
