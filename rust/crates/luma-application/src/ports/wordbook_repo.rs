use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordEntry {
    pub id: i64,
    pub term: String,
    pub phonetic: String,
    pub meaning: String,
    pub example: String,
    pub category: String,
    pub familiarity: String,
    pub review_stage: i64,
    pub review_count: i64,
    pub wrong_count: i64,
    pub last_review_at: String,
    pub next_review_at: String,
    pub mastered_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordContentInput {
    pub term: String,
    pub phonetic: String,
    pub meaning: String,
    pub example: String,
    pub category: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WordbookStatsView {
    pub total: i64,
    pub due: i64,
    pub new_count: i64,
    pub wrong: i64,
    pub mastered: i64,
    pub goal: i64,
    pub reviewed_today: i64,
    pub remaining_goal: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ContentImportReport {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct WordbookRepoError(pub String);

impl WordbookRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait WordbookRepository: Send + Sync {
    fn get(&self, id: i64) -> Result<Option<WordEntry>, WordbookRepoError>;
    fn get_by_term(&self, term: &str) -> Result<Option<WordEntry>, WordbookRepoError>;
    fn list_due(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError>;
    fn list_new(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError>;
    fn list_wrong(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError>;
    fn stats(&self) -> Result<WordbookStatsView, WordbookRepoError>;
    fn daily_goal(&self) -> Result<i64, WordbookRepoError>;
    fn set_daily_goal(&self, value: i64) -> Result<(), WordbookRepoError>;
    fn upsert_content(&self, content: &WordContentInput) -> Result<bool, WordbookRepoError>;
    fn upsert_contents(
        &self,
        rows: &[WordContentInput],
    ) -> Result<ContentImportReport, WordbookRepoError>;
    fn delete(&self, id: i64) -> Result<(), WordbookRepoError>;
    fn review(&self, id: i64, familiarity: &str) -> Result<WordEntry, WordbookRepoError>;
    fn set_mastered(&self, id: i64, mastered: bool) -> Result<WordEntry, WordbookRepoError>;
    fn backup(&self) -> Result<std::path::PathBuf, WordbookRepoError>;
}
