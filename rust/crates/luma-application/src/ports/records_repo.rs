//! Records SQLite port (personal media/content log).

use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordCategory {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub source_file: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordEntry {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    pub rating: Option<i64>,
    pub note: String,
    pub source_file: String,
    pub source_key: String,
    pub source_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecordsStatsView {
    pub categories: i64,
    pub records: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecordImportPreviewView {
    pub files_found: usize,
    pub categories: usize,
    pub records: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecordImportReportView {
    pub categories_upserted: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub migration_id: Option<String>,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct RecordsRepoError(pub String);

impl RecordsRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait RecordsRepository: Send + Sync {
    fn stats(&self) -> Result<RecordsStatsView, RecordsRepoError>;
    fn list_categories(&self) -> Result<Vec<RecordCategory>, RecordsRepoError>;
    fn get(&self, id: i64) -> Result<Option<RecordEntry>, RecordsRepoError>;
    fn get_by_category_and_name(
        &self,
        category: &str,
        name: &str,
    ) -> Result<Option<RecordEntry>, RecordsRepoError>;
    fn list_by_category(
        &self,
        category: &str,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError>;
    fn search(
        &self,
        query: &str,
        category_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError>;
    fn insert(
        &self,
        category: &str,
        name: &str,
        rating: Option<i64>,
        note: &str,
    ) -> Result<RecordEntry, RecordsRepoError>;
    fn update(
        &self,
        id: i64,
        name: Option<&str>,
        rating: Option<Option<i64>>,
        note: Option<&str>,
        category: Option<&str>,
    ) -> Result<RecordEntry, RecordsRepoError>;
    fn set_rating(&self, id: i64, rating: Option<i64>) -> Result<RecordEntry, RecordsRepoError>;
    fn set_note(&self, id: i64, note: &str) -> Result<RecordEntry, RecordsRepoError>;
    fn delete(&self, id: i64) -> Result<(), RecordsRepoError>;
    fn preview_import(&self, root: &Path) -> Result<RecordImportPreviewView, RecordsRepoError>;
    fn import_dir(&self, root: &Path) -> Result<RecordImportReportView, RecordsRepoError>;
    fn backup(&self) -> Result<std::path::PathBuf, RecordsRepoError>;
}
