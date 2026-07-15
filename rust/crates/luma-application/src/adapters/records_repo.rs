use crate::ports::{
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView,
};
use async_trait::async_trait;
use luma_storage::RecordsStore;
use std::path::Path;
use std::sync::Arc;

pub struct SqliteRecordsRepository {
    store: Arc<RecordsStore>,
}

impl SqliteRecordsRepository {
    pub fn new(store: Arc<RecordsStore>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &RecordsStore {
        &self.store
    }
}

fn map_category(r: luma_storage::RecordCategoryRow) -> RecordCategory {
    RecordCategory {
        id: r.id,
        name: r.name,
        sort_order: r.sort_order,
        source_file: r.source_file,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn map_record(r: luma_storage::RecordRow) -> RecordEntry {
    RecordEntry {
        id: r.id,
        category_id: r.category_id,
        category_name: r.category_name,
        name: r.name,
        rating: r.rating,
        note: r.note,
        source_file: r.source_file,
        source_key: r.source_key,
        source_hash: r.source_hash,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[async_trait]
impl RecordsRepository for SqliteRecordsRepository {
    fn stats(&self) -> Result<RecordsStatsView, RecordsRepoError> {
        self.store
            .stats()
            .map(|s| RecordsStatsView {
                categories: s.categories,
                records: s.records,
            })
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn list_categories(&self) -> Result<Vec<RecordCategory>, RecordsRepoError> {
        self.store
            .list_categories()
            .map(|rows| rows.into_iter().map(map_category).collect())
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn get(&self, id: i64) -> Result<Option<RecordEntry>, RecordsRepoError> {
        self.store
            .get(id)
            .map(|o| o.map(map_record))
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn get_by_category_and_name(
        &self,
        category: &str,
        name: &str,
    ) -> Result<Option<RecordEntry>, RecordsRepoError> {
        self.store
            .get_by_category_and_name(category, name)
            .map(|o| o.map(map_record))
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn list_by_category(
        &self,
        category: &str,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError> {
        self.store
            .list_by_category(category, limit)
            .map(|rows| rows.into_iter().map(map_record).collect())
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn search(
        &self,
        query: &str,
        category_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError> {
        self.store
            .search(query, category_filter, limit)
            .map(|rows| rows.into_iter().map(map_record).collect())
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn insert(
        &self,
        category: &str,
        name: &str,
        rating: Option<i64>,
        note: &str,
    ) -> Result<RecordEntry, RecordsRepoError> {
        self.store
            .insert_record(category, name, rating, note)
            .map(map_record)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn update(
        &self,
        id: i64,
        name: Option<&str>,
        rating: Option<Option<i64>>,
        note: Option<&str>,
        category: Option<&str>,
    ) -> Result<RecordEntry, RecordsRepoError> {
        self.store
            .update_record(id, name, rating, note, category)
            .map(map_record)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn set_rating(&self, id: i64, rating: Option<i64>) -> Result<RecordEntry, RecordsRepoError> {
        self.store
            .set_rating(id, rating)
            .map(map_record)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn set_note(&self, id: i64, note: &str) -> Result<RecordEntry, RecordsRepoError> {
        self.store
            .set_note(id, note)
            .map(map_record)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn delete(&self, id: i64) -> Result<(), RecordsRepoError> {
        self.store
            .delete(id)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn preview_import(&self, root: &Path) -> Result<RecordImportPreviewView, RecordsRepoError> {
        self.store
            .preview_import_dir(root)
            .map(|p| RecordImportPreviewView {
                files_found: p.files_found,
                categories: p.categories,
                records: p.records,
                warnings: p.warnings,
                errors: p.errors,
            })
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }

    fn import_dir(&self, root: &Path) -> Result<RecordImportReportView, RecordsRepoError> {
        let report = luma_storage::import_records_with_ledger(root, &self.store, true)
            .map_err(|e| RecordsRepoError::msg(e.to_string()))?;
        let apply = report
            .apply
            .ok_or_else(|| RecordsRepoError::msg("records import did not apply"))?;
        Ok(RecordImportReportView {
            categories_upserted: apply.categories_upserted,
            inserted: apply.inserted,
            skipped: apply.skipped,
            warnings: apply.warnings,
            errors: apply.errors,
            migration_id: report.migration.map(|m| m.migration_id),
        })
    }

    fn backup(&self) -> Result<std::path::PathBuf, RecordsRepoError> {
        self.store
            .backup()
            .map_err(|e| RecordsRepoError::msg(e.to_string()))
    }
}
