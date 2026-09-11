use crate::ports::{
    ContentImportReport, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
use async_trait::async_trait;
use luma_storage::{WordContent, WordbookStore};
use std::sync::Arc;

pub struct SqliteWordbookRepository {
    store: Arc<WordbookStore>,
}

impl SqliteWordbookRepository {
    pub fn new(store: Arc<WordbookStore>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &WordbookStore {
        &self.store
    }
}

fn map_entry(r: luma_storage::WordRow) -> WordEntry {
    WordEntry {
        id: r.id,
        term: r.term,
        phonetic: r.phonetic,
        meaning: r.meaning,
        example: r.example,
        category: r.category,
        familiarity: r.familiarity,
        review_stage: r.review_stage,
        review_count: r.review_count,
        wrong_count: r.wrong_count,
        last_review_at: r.last_review_at,
        next_review_at: r.next_review_at,
        mastered_at: r.mastered_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn map_content(c: &WordContentInput) -> WordContent {
    WordContent {
        term: c.term.clone(),
        phonetic: c.phonetic.clone(),
        meaning: c.meaning.clone(),
        example: c.example.clone(),
        category: c.category.clone(),
    }
}

#[async_trait]
impl WordbookRepository for SqliteWordbookRepository {
    fn get(&self, id: i64) -> Result<Option<WordEntry>, WordbookRepoError> {
        self.store
            .get(id)
            .map(|o| o.map(map_entry))
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn get_by_term(&self, term: &str) -> Result<Option<WordEntry>, WordbookRepoError> {
        self.store
            .get_by_term(term)
            .map(|o| o.map(map_entry))
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn list_due(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        self.store
            .list_due(limit)
            .map(|rows| rows.into_iter().map(map_entry).collect())
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn list_new(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        self.store
            .list_new(limit)
            .map(|rows| rows.into_iter().map(map_entry).collect())
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn list_wrong(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        self.store
            .list_wrong(limit)
            .map(|rows| rows.into_iter().map(map_entry).collect())
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        self.store
            .search(query, limit)
            .map(|rows| rows.into_iter().map(map_entry).collect())
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn stats(&self) -> Result<WordbookStatsView, WordbookRepoError> {
        self.store
            .stats()
            .map(|s| WordbookStatsView {
                total: s.total,
                due: s.due,
                new_count: s.new_count,
                wrong: s.wrong,
                mastered: s.mastered,
                goal: s.goal,
                reviewed_today: s.reviewed_today,
                remaining_goal: s.remaining_goal,
            })
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn daily_goal(&self) -> Result<i64, WordbookRepoError> {
        self.store
            .daily_goal()
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn set_daily_goal(&self, value: i64) -> Result<(), WordbookRepoError> {
        self.store
            .set_daily_goal(value)
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn upsert_content(&self, content: &WordContentInput) -> Result<bool, WordbookRepoError> {
        self.store
            .upsert_content(&map_content(content))
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn upsert_contents(
        &self,
        rows: &[WordContentInput],
    ) -> Result<ContentImportReport, WordbookRepoError> {
        let mapped: Vec<_> = rows.iter().map(map_content).collect();
        self.store
            .upsert_contents(&mapped)
            .map(|r| ContentImportReport {
                inserted: r.inserted,
                updated: r.updated,
                skipped: r.skipped,
            })
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn delete(&self, id: i64) -> Result<(), WordbookRepoError> {
        self.store
            .delete(id)
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn review(&self, id: i64, familiarity: &str) -> Result<WordEntry, WordbookRepoError> {
        self.store
            .review(id, familiarity)
            .map(map_entry)
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn set_mastered(&self, id: i64, mastered: bool) -> Result<WordEntry, WordbookRepoError> {
        self.store
            .set_mastered(id, mastered)
            .map(map_entry)
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }

    fn backup(&self) -> Result<std::path::PathBuf, WordbookRepoError> {
        self.store
            .backup()
            .map_err(|e| WordbookRepoError::msg(e.to_string()))
    }
}
