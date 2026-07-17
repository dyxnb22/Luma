use crate::ports::{
    ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, ContentImportReport,
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesScanReport,
    NotesScanStatusView, NotesSearchHit, QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository,
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView, ResolvedSshHost, SnippetEntry, SnippetsRepoError,
    SnippetsRepository, SshConfigError, SshConfigPort, SshConfigState, SshHostMeta,
    SshMetaRepoError, SshMetaRepository, TimerEntry, TimersRepoError, TimersRepository,
    WordContentInput, WordEntry, WordbookRepoError, WordbookRepository, WordbookStatsView,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// In-memory clipboard history for module tests (no SQLite).
#[derive(Default)]
pub struct MemoryClipboardHistory {
    rows: Mutex<Vec<ClipboardEntry>>,
    next_id: Mutex<i64>,
}

impl MemoryClipboardHistory {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ClipboardHistoryRepository for MemoryClipboardHistory {
    fn list_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError> {
        let mut rows = self.rows.lock().expect("lock").clone();
        rows.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.created_at.cmp(&a.created_at))
        });
        Ok(rows.into_iter().skip(offset).take(limit).collect())
    }

    fn search_text(
        &self,
        needle: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardEntry>, ClipboardRepoError> {
        let needle = needle.to_lowercase();
        let mut rows: Vec<_> = self
            .rows
            .lock()
            .expect("lock")
            .iter()
            .filter(|r| r.text.to_lowercase().contains(&needle))
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.created_at.cmp(&a.created_at))
        });
        rows.truncate(limit);
        Ok(rows)
    }

    fn latest_by_created(&self) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
        Ok(self
            .rows
            .lock()
            .expect("lock")
            .iter()
            .max_by_key(|r| r.created_at)
            .cloned())
    }

    fn purge_older_than_days(&self, days: u32) -> Result<usize, ClipboardRepoError> {
        let cutoff = chrono_now() - i64::from(days) * 86_400;
        let mut rows = self.rows.lock().expect("lock");
        let before = rows.len();
        rows.retain(|r| r.pinned || r.created_at >= cutoff);
        Ok(before - rows.len())
    }

    fn insert(&self, text: &str, pinned: bool) -> Result<i64, ClipboardRepoError> {
        let mut next = self.next_id.lock().expect("lock");
        *next += 1;
        let id = *next;
        self.rows.lock().expect("lock").push(ClipboardEntry {
            id,
            text: text.into(),
            pinned,
            created_at: chrono_now(),
        });
        Ok(id)
    }

    fn get(&self, id: i64) -> Result<Option<ClipboardEntry>, ClipboardRepoError> {
        Ok(self
            .rows
            .lock()
            .expect("lock")
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    fn delete(&self, id: i64) -> Result<(), ClipboardRepoError> {
        self.rows.lock().expect("lock").retain(|r| r.id != id);
        Ok(())
    }

    fn set_pinned(&self, id: i64, pinned: bool) -> Result<(), ClipboardRepoError> {
        if let Some(row) = self
            .rows
            .lock()
            .expect("lock")
            .iter_mut()
            .find(|r| r.id == id)
        {
            row.pinned = pinned;
        }
        Ok(())
    }

    fn clear_unpinned(&self) -> Result<usize, ClipboardRepoError> {
        let mut rows = self.rows.lock().expect("lock");
        let before = rows.len();
        rows.retain(|r| r.pinned);
        Ok(before - rows.len())
    }
}

/// In-memory quicklinks for module tests.
#[derive(Default)]
pub struct MemoryQuicklinksRepository {
    rows: Mutex<BTreeMap<String, String>>,
}

impl MemoryQuicklinksRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl QuicklinksRepository for MemoryQuicklinksRepository {
    fn list(&self) -> Result<Vec<QuicklinkEntry>, QuicklinksRepoError> {
        Ok(self
            .rows
            .lock()
            .expect("lock")
            .iter()
            .map(|(trigger, url)| QuicklinkEntry {
                trigger: trigger.clone(),
                url: url.clone(),
            })
            .collect())
    }

    fn upsert(&self, trigger: &str, url: &str) -> Result<(), QuicklinksRepoError> {
        self.rows
            .lock()
            .expect("lock")
            .insert(trigger.into(), url.into());
        Ok(())
    }

    fn delete(&self, trigger: &str) -> Result<(), QuicklinksRepoError> {
        self.rows.lock().expect("lock").remove(trigger);
        Ok(())
    }
}

/// In-memory snippets for module tests.
#[derive(Default)]
pub struct MemorySnippetsRepository {
    rows: Mutex<BTreeMap<String, String>>,
}

impl MemorySnippetsRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SnippetsRepository for MemorySnippetsRepository {
    fn list(&self) -> Result<Vec<SnippetEntry>, SnippetsRepoError> {
        Ok(self
            .rows
            .lock()
            .expect("lock")
            .iter()
            .map(|(trigger, body)| SnippetEntry {
                trigger: trigger.clone(),
                body: body.clone(),
            })
            .collect())
    }

    fn get(&self, trigger: &str) -> Result<Option<SnippetEntry>, SnippetsRepoError> {
        Ok(self
            .rows
            .lock()
            .expect("lock")
            .get(trigger)
            .map(|body| SnippetEntry {
                trigger: trigger.into(),
                body: body.clone(),
            }))
    }

    fn upsert(&self, trigger: &str, body: &str) -> Result<(), SnippetsRepoError> {
        self.rows
            .lock()
            .expect("lock")
            .insert(trigger.into(), body.into());
        Ok(())
    }

    fn delete(&self, trigger: &str) -> Result<(), SnippetsRepoError> {
        self.rows.lock().expect("lock").remove(trigger);
        Ok(())
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Clone, Debug)]
struct MemNote {
    relative_path: String,
    title: String,
    body: String,
    mtime_unix: i64,
    size_bytes: i64,
}

/// Pure in-memory notes index for module tests (no SQLite and no host filesystem).
///
/// Call [`Self::insert_document`] to seed it. Keeping this fake memory-only matters because the
/// production application crate must not hide filesystem access behind a type named "memory".
pub struct MemoryNotesIndex {
    docs: Mutex<BTreeMap<String, MemNote>>,
    status: Mutex<NotesScanStatusView>,
}

impl Default for MemoryNotesIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryNotesIndex {
    pub fn new() -> Self {
        Self {
            docs: Mutex::new(BTreeMap::new()),
            status: Mutex::new(NotesScanStatusView::Idle),
        }
    }

    pub fn insert_document(
        &self,
        relative_path: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        let relative_path = relative_path.into();
        let body = body.into();
        self.docs.lock().expect("lock").insert(
            relative_path.clone(),
            MemNote {
                relative_path,
                title: title.into(),
                size_bytes: body.len() as i64,
                body,
                mtime_unix: 0,
            },
        );
    }
}

#[async_trait]
impl NotesIndexRepository for MemoryNotesIndex {
    fn search(&self, query: &str, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let docs = self.docs.lock().expect("lock");
        let mut hits: Vec<_> = docs
            .values()
            .filter(|d| {
                d.title.to_lowercase().contains(&q)
                    || d.body.to_lowercase().contains(&q)
                    || d.relative_path.to_lowercase().contains(&q)
            })
            .map(|d| NotesSearchHit {
                relative_path: d.relative_path.clone(),
                title: d.title.clone(),
                snippet: d.relative_path.clone(),
                rank: 0.0,
            })
            .take(limit)
            .collect();
        hits.truncate(limit);
        Ok(hits)
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError> {
        let mut docs: Vec<_> = self.docs.lock().expect("lock").values().cloned().collect();
        docs.sort_by(|a, b| {
            b.mtime_unix
                .cmp(&a.mtime_unix)
                .then(a.relative_path.cmp(&b.relative_path))
        });
        Ok(docs
            .into_iter()
            .take(limit)
            .map(|d| NotesSearchHit {
                relative_path: d.relative_path.clone(),
                title: d.title,
                snippet: d.relative_path,
                rank: 0.0,
            })
            .collect())
    }

    fn document_count(&self) -> Result<usize, NotesIndexError> {
        Ok(self.docs.lock().expect("lock").len())
    }

    fn fts_count(&self) -> Result<usize, NotesIndexError> {
        self.document_count()
    }

    fn get_document(&self, relative_path: &str) -> Result<Option<NotesDocument>, NotesIndexError> {
        Ok(self
            .docs
            .lock()
            .expect("lock")
            .get(relative_path)
            .map(|d| NotesDocument {
                relative_path: d.relative_path.clone(),
                title: d.title.clone(),
                file_name: PathBuf::from(&d.relative_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("note.md")
                    .to_string(),
                size_bytes: d.size_bytes,
                mtime_unix: d.mtime_unix,
                tags: Vec::new(),
                outbound: Vec::new(),
                backlinks: Vec::new(),
            }))
    }

    fn list_issues(&self) -> Result<Vec<NotesIssue>, NotesIndexError> {
        Ok(Vec::new())
    }

    fn scan_status(&self) -> NotesScanStatusView {
        self.status.lock().expect("lock").clone()
    }

    fn full_scan(
        &self,
        _root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        if cancel
            .as_ref()
            .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        {
            return Ok(NotesScanReport {
                mode: "full".into(),
                processed: 0,
                errors: 0,
                pruned: 0,
                cancelled: true,
            });
        }
        let n = self.docs.lock().expect("lock").len();
        *self.status.lock().expect("lock") = NotesScanStatusView::Completed {
            mode: "full".into(),
            processed: n,
            total: n,
            errors: 0,
        };
        Ok(NotesScanReport {
            mode: "full".into(),
            processed: n,
            errors: 0,
            pruned: 0,
            cancelled: false,
        })
    }

    fn incremental_check(
        &self,
        root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        self.full_scan(root, cancel)
    }

    fn rebuild(
        &self,
        root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        self.full_scan(root, cancel)
    }
}

/// In-memory wordbook for module tests (no SQLite).
#[derive(Default)]
pub struct MemoryWordbookRepository {
    words: Mutex<Vec<WordEntry>>,
    next_id: Mutex<i64>,
    daily_goal: Mutex<i64>,
}

impl MemoryWordbookRepository {
    pub fn new() -> Self {
        Self {
            words: Mutex::new(Vec::new()),
            next_id: Mutex::new(0),
            daily_goal: Mutex::new(30),
        }
    }

    fn now() -> String {
        format!("{}", chrono_now())
    }
}

#[async_trait]
impl WordbookRepository for MemoryWordbookRepository {
    fn get(&self, id: i64) -> Result<Option<WordEntry>, WordbookRepoError> {
        Ok(self
            .words
            .lock()
            .expect("lock")
            .iter()
            .find(|w| w.id == id)
            .cloned())
    }

    fn get_by_term(&self, term: &str) -> Result<Option<WordEntry>, WordbookRepoError> {
        Ok(self
            .words
            .lock()
            .expect("lock")
            .iter()
            .find(|w| w.term == term)
            .cloned())
    }

    fn list_due(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        let now = Self::now();
        let mut rows: Vec<_> = self
            .words
            .lock()
            .expect("lock")
            .iter()
            .filter(|w| {
                w.mastered_at.is_empty()
                    && w.review_count > 0
                    && (w.next_review_at.is_empty() || w.next_review_at <= now)
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            (b.wrong_count >= 2)
                .cmp(&(a.wrong_count >= 2))
                .then(a.next_review_at.cmp(&b.next_review_at))
                .then(b.wrong_count.cmp(&a.wrong_count))
        });
        rows.truncate(limit);
        Ok(rows)
    }

    fn list_new(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        let mut rows: Vec<_> = self
            .words
            .lock()
            .expect("lock")
            .iter()
            .filter(|w| w.mastered_at.is_empty() && w.review_count == 0)
            .cloned()
            .collect();
        rows.sort_by_key(|a| a.id);
        rows.truncate(limit);
        Ok(rows)
    }

    fn list_wrong(&self, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        let mut rows: Vec<_> = self
            .words
            .lock()
            .expect("lock")
            .iter()
            .filter(|w| w.mastered_at.is_empty() && w.review_count > 0 && w.wrong_count > 0)
            .cloned()
            .collect();
        rows.sort_by_key(|b| std::cmp::Reverse(b.wrong_count));
        rows.truncate(limit);
        Ok(rows)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<WordEntry>, WordbookRepoError> {
        let q = query.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .words
            .lock()
            .expect("lock")
            .iter()
            .filter(|w| {
                q.is_empty()
                    || w.term.to_lowercase().contains(&q)
                    || w.meaning.to_lowercase().contains(&q)
                    || w.example.to_lowercase().contains(&q)
                    || w.category.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        rows.truncate(limit);
        Ok(rows)
    }

    fn stats(&self) -> Result<WordbookStatsView, WordbookRepoError> {
        let words = self.words.lock().expect("lock");
        let goal = *self.daily_goal.lock().expect("lock");
        let now = Self::now();
        let now_secs = chrono_now();
        let today_start = now_secs - (now_secs % 86_400);
        let total = words.len() as i64;
        let due = words
            .iter()
            .filter(|w| {
                w.mastered_at.is_empty()
                    && w.review_count > 0
                    && (w.next_review_at.is_empty() || w.next_review_at <= now)
            })
            .count() as i64;
        let new_count = words
            .iter()
            .filter(|w| w.mastered_at.is_empty() && w.review_count == 0)
            .count() as i64;
        let wrong = words
            .iter()
            .filter(|w| w.mastered_at.is_empty() && w.review_count > 0 && w.wrong_count > 0)
            .count() as i64;
        let mastered = words
            .iter()
            .filter(|w| !w.mastered_at.is_empty() || w.familiarity == "mastered")
            .count() as i64;
        let reviewed_today = words
            .iter()
            .filter(|w| {
                w.last_review_at
                    .parse::<i64>()
                    .ok()
                    .is_some_and(|t| t >= today_start)
            })
            .count() as i64;
        Ok(WordbookStatsView {
            total,
            due,
            new_count,
            wrong,
            mastered,
            goal,
            reviewed_today,
            remaining_goal: (goal - reviewed_today).max(0),
        })
    }

    fn daily_goal(&self) -> Result<i64, WordbookRepoError> {
        Ok(*self.daily_goal.lock().expect("lock"))
    }

    fn set_daily_goal(&self, value: i64) -> Result<(), WordbookRepoError> {
        *self.daily_goal.lock().expect("lock") = value.max(1);
        Ok(())
    }

    fn upsert_content(&self, content: &WordContentInput) -> Result<bool, WordbookRepoError> {
        let term = content.term.trim();
        if term.is_empty() {
            return Err(WordbookRepoError::msg("term is required"));
        }
        let mut words = self.words.lock().expect("lock");
        if let Some(existing) = words.iter_mut().find(|w| w.term == term) {
            existing.phonetic = content.phonetic.clone();
            existing.meaning = content.meaning.clone();
            existing.example = content.example.clone();
            existing.category = content.category.clone();
            existing.updated_at = Self::now();
            return Ok(false);
        }
        let mut next = self.next_id.lock().expect("lock");
        *next += 1;
        let id = *next;
        let now = Self::now();
        words.push(WordEntry {
            id,
            term: term.into(),
            phonetic: content.phonetic.clone(),
            meaning: content.meaning.clone(),
            example: content.example.clone(),
            category: content.category.clone(),
            familiarity: "unknown".into(),
            review_stage: 0,
            review_count: 0,
            wrong_count: 0,
            last_review_at: String::new(),
            next_review_at: now.clone(),
            mastered_at: String::new(),
            created_at: now.clone(),
            updated_at: now,
        });
        Ok(true)
    }

    fn upsert_contents(
        &self,
        rows: &[WordContentInput],
    ) -> Result<ContentImportReport, WordbookRepoError> {
        let mut report = ContentImportReport::default();
        for row in rows {
            if row.term.trim().is_empty() {
                report.skipped += 1;
                continue;
            }
            if self.upsert_content(row)? {
                report.inserted += 1;
            } else {
                report.updated += 1;
            }
        }
        Ok(report)
    }

    fn delete(&self, id: i64) -> Result<(), WordbookRepoError> {
        self.words.lock().expect("lock").retain(|w| w.id != id);
        Ok(())
    }

    fn review(&self, id: i64, familiarity: &str) -> Result<WordEntry, WordbookRepoError> {
        let mut words = self.words.lock().expect("lock");
        let word = words
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| WordbookRepoError::msg(format!("word {id} not found")))?;
        if !word.mastered_at.is_empty() || word.familiarity == "mastered" {
            return Err(WordbookRepoError::msg(format!(
                "word {id} is mastered; unmaster before reviewing"
            )));
        }
        let wrong = if familiarity == "unknown" {
            word.wrong_count + 1
        } else {
            word.wrong_count
        };
        let stage = match familiarity {
            "known" => (word.review_stage + 1).min(9),
            "fuzzy" => word.review_stage,
            "unknown" => 0,
            other => {
                return Err(WordbookRepoError::msg(format!(
                    "invalid familiarity: {other}"
                )))
            }
        };
        word.familiarity = familiarity.into();
        word.review_stage = stage;
        word.review_count += 1;
        word.wrong_count = wrong;
        word.last_review_at = Self::now();
        word.next_review_at = Self::now();
        word.updated_at = Self::now();
        word.mastered_at.clear();
        Ok(word.clone())
    }

    fn set_mastered(&self, id: i64, mastered: bool) -> Result<WordEntry, WordbookRepoError> {
        let mut words = self.words.lock().expect("lock");
        let word = words
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| WordbookRepoError::msg(format!("word {id} not found")))?;
        if mastered {
            word.familiarity = "mastered".into();
            word.mastered_at = Self::now();
            word.next_review_at = "9999-12-31T00:00:00Z".into();
            word.review_stage = 9;
            word.review_count = word.review_count.max(9);
            word.last_review_at = Self::now();
        } else {
            word.familiarity = "unknown".into();
            word.mastered_at.clear();
            word.review_stage = 0;
            word.review_count = 0;
            word.last_review_at.clear();
            word.next_review_at = Self::now();
        }
        word.updated_at = Self::now();
        Ok(word.clone())
    }

    fn backup(&self) -> Result<std::path::PathBuf, WordbookRepoError> {
        Ok(std::path::PathBuf::from(
            "/tmp/wordbook-memory-backup.sqlite",
        ))
    }
}

/// In-memory records store for module tests (no SQLite).
#[derive(Default)]
pub struct MemoryRecordsRepository {
    categories: Mutex<Vec<RecordCategory>>,
    records: Mutex<Vec<RecordEntry>>,
    next_cat_id: Mutex<i64>,
    next_id: Mutex<i64>,
}

impl MemoryRecordsRepository {
    pub fn new() -> Self {
        Self::default()
    }

    fn now() -> String {
        format!("{}", chrono_now())
    }

    fn norm(name: &str) -> String {
        name.split_whitespace()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn add_category(&self, category: &str) {
        let mut cats = self.categories.lock().expect("lock");
        if cats.iter().any(|c| c.name == category) {
            return;
        }
        let mut next = self.next_cat_id.lock().expect("lock");
        *next += 1;
        let now = Self::now();
        cats.push(RecordCategory {
            id: *next,
            name: category.into(),
            sort_order: *next,
            source_file: String::new(),
            created_at: now.clone(),
            updated_at: now,
        });
    }
}

#[async_trait]
impl RecordsRepository for MemoryRecordsRepository {
    fn stats(&self) -> Result<RecordsStatsView, RecordsRepoError> {
        Ok(RecordsStatsView {
            categories: self.categories.lock().expect("lock").len() as i64,
            records: self.records.lock().expect("lock").len() as i64,
        })
    }

    fn list_categories(&self) -> Result<Vec<RecordCategory>, RecordsRepoError> {
        Ok(self.categories.lock().expect("lock").clone())
    }

    fn get(&self, id: i64) -> Result<Option<RecordEntry>, RecordsRepoError> {
        Ok(self
            .records
            .lock()
            .expect("lock")
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    fn get_by_category_and_name(
        &self,
        category: &str,
        name: &str,
    ) -> Result<Option<RecordEntry>, RecordsRepoError> {
        let norm = Self::norm(name);
        Ok(self
            .records
            .lock()
            .expect("lock")
            .iter()
            .find(|r| r.category_name == category && Self::norm(&r.name) == norm)
            .cloned())
    }

    fn list_by_category(
        &self,
        category: &str,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError> {
        let mut rows: Vec<_> = self
            .records
            .lock()
            .expect("lock")
            .iter()
            .filter(|r| r.category_name == category)
            .cloned()
            .collect();
        rows.truncate(limit);
        Ok(rows)
    }

    fn search(
        &self,
        query: &str,
        category_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordEntry>, RecordsRepoError> {
        let q = query.to_lowercase();
        let mut rows: Vec<_> = self
            .records
            .lock()
            .expect("lock")
            .iter()
            .filter(|r| {
                if let Some(cat) = category_filter {
                    if r.category_name != cat {
                        return false;
                    }
                }
                r.name.to_lowercase().contains(&q)
                    || r.note.to_lowercase().contains(&q)
                    || r.category_name.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        rows.truncate(limit);
        Ok(rows)
    }

    fn insert(
        &self,
        category: &str,
        name: &str,
        rating: Option<i64>,
        note: &str,
    ) -> Result<RecordEntry, RecordsRepoError> {
        if let Some(rating) = rating {
            if !(1..=10).contains(&rating) {
                return Err(RecordsRepoError::msg("rating must be between 1 and 10"));
            }
        }
        let category = category.trim();
        if category.is_empty() {
            return Err(RecordsRepoError::msg("category is empty"));
        }
        let norm = Self::norm(name);
        if norm.is_empty() {
            return Err(RecordsRepoError::msg("name is empty"));
        }
        let mut records = self.records.lock().expect("lock");
        if records
            .iter()
            .any(|r| r.category_name == category && Self::norm(&r.name) == norm)
        {
            return Err(RecordsRepoError::msg("duplicate name"));
        }
        let cats = self.categories.lock().expect("lock");
        if !cats.iter().any(|c| c.name == category) {
            return Err(RecordsRepoError::msg(
                "category does not exist; import categories first",
            ));
        }
        let cat_id = cats
            .iter()
            .find(|c| c.name == category)
            .map(|c| c.id)
            .unwrap_or(1);
        let mut next = self.next_id.lock().expect("lock");
        *next += 1;
        let now = Self::now();
        let row = RecordEntry {
            id: *next,
            category_id: cat_id,
            category_name: category.into(),
            name: name.trim().into(),
            rating,
            note: note.into(),
            source_file: String::new(),
            source_key: String::new(),
            source_hash: String::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        records.push(row.clone());
        Ok(row)
    }

    fn update(
        &self,
        id: i64,
        name: Option<&str>,
        rating: Option<Option<i64>>,
        note: Option<&str>,
        category: Option<&str>,
    ) -> Result<RecordEntry, RecordsRepoError> {
        let mut records = self.records.lock().expect("lock");
        let row = records
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| RecordsRepoError::msg(format!("record {id} not found")))?;
        if let Some(rating) = rating {
            if let Some(rating) = rating {
                if !(1..=10).contains(&rating) {
                    return Err(RecordsRepoError::msg("rating must be between 1 and 10"));
                }
            }
            row.rating = rating;
        }
        if let Some(cat) = category {
            let cat = cat.trim();
            if !self
                .categories
                .lock()
                .expect("lock")
                .iter()
                .any(|c| c.name == cat)
            {
                return Err(RecordsRepoError::msg(
                    "category does not exist; import categories first",
                ));
            }
            row.category_name = cat.into();
        }
        if let Some(n) = name {
            row.name = n.trim().into();
        }
        if let Some(n) = note {
            row.note = n.into();
        }
        row.updated_at = Self::now();
        Ok(row.clone())
    }

    fn set_rating(&self, id: i64, rating: Option<i64>) -> Result<RecordEntry, RecordsRepoError> {
        self.update(id, None, Some(rating), None, None)
    }

    fn set_note(&self, id: i64, note: &str) -> Result<RecordEntry, RecordsRepoError> {
        self.update(id, None, None, Some(note), None)
    }

    fn delete(&self, id: i64) -> Result<(), RecordsRepoError> {
        let mut records = self.records.lock().expect("lock");
        let len = records.len();
        records.retain(|r| r.id != id);
        if records.len() == len {
            return Err(RecordsRepoError::msg(format!("record {id} not found")));
        }
        Ok(())
    }

    fn preview_import(&self, _root: &Path) -> Result<RecordImportPreviewView, RecordsRepoError> {
        Ok(RecordImportPreviewView::default())
    }

    fn import_dir(&self, _root: &Path) -> Result<RecordImportReportView, RecordsRepoError> {
        Ok(RecordImportReportView::default())
    }

    fn backup(&self) -> Result<std::path::PathBuf, RecordsRepoError> {
        Ok(PathBuf::from("/tmp/records-memory-backup.sqlite"))
    }
}

/// In-memory SSH config for module tests.
pub struct FakeSshConfigPort {
    state: Mutex<SshConfigState>,
    aliases: Mutex<Vec<String>>,
    resolved: Mutex<BTreeMap<String, ResolvedSshHost>>,
    ssh_available: Mutex<bool>,
    sftp_available: Mutex<bool>,
}

impl Default for FakeSshConfigPort {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeSshConfigPort {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(SshConfigState::Found),
            aliases: Mutex::new(Vec::new()),
            resolved: Mutex::new(BTreeMap::new()),
            ssh_available: Mutex::new(true),
            sftp_available: Mutex::new(true),
        }
    }

    pub fn with_aliases(self, aliases: Vec<&str>, resolved: Vec<ResolvedSshHost>) -> Self {
        *self.aliases.lock().expect("lock") = aliases.into_iter().map(str::to_string).collect();
        let mut map = BTreeMap::new();
        for host in resolved {
            map.insert(host.alias.clone(), host);
        }
        *self.resolved.lock().expect("lock") = map;
        self
    }

    pub fn set_state(&self, state: SshConfigState) {
        *self.state.lock().expect("lock") = state;
    }

    pub fn set_sftp_available(&self, available: bool) {
        *self.sftp_available.lock().expect("lock") = available;
    }
}

impl SshConfigPort for FakeSshConfigPort {
    fn config_state(&self) -> SshConfigState {
        self.state.lock().expect("lock").clone()
    }

    fn list_aliases(&self) -> Result<Vec<String>, SshConfigError> {
        match self.config_state() {
            SshConfigState::NotConfigured => Err(SshConfigError::msg("ssh config not found")),
            SshConfigState::Unavailable(reason) => Err(SshConfigError::msg(reason)),
            SshConfigState::Found => Ok(self.aliases.lock().expect("lock").clone()),
        }
    }

    fn resolve(&self, alias: &str) -> Result<ResolvedSshHost, SshConfigError> {
        if alias.trim().starts_with('-') {
            return Err(SshConfigError::msg(format!(
                "refusing ssh host alias that looks like a flag: {alias}"
            )));
        }
        self.resolved
            .lock()
            .expect("lock")
            .get(alias)
            .cloned()
            .ok_or_else(|| SshConfigError::msg(format!("unknown alias: {alias}")))
    }

    fn ssh_available(&self) -> bool {
        *self.ssh_available.lock().expect("lock")
    }

    fn sftp_available(&self) -> bool {
        *self.sftp_available.lock().expect("lock")
    }
}

/// In-memory SSH host metadata for module tests.
#[derive(Default)]
pub struct MemorySshMetaRepository {
    rows: Mutex<BTreeMap<String, SshHostMeta>>,
    list_error: Mutex<Option<String>>,
}

impl MemorySshMetaRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_list_error(&self, error: Option<String>) {
        *self.list_error.lock().expect("lock") = error;
    }
}

#[async_trait]
impl SshMetaRepository for MemorySshMetaRepository {
    fn list(&self) -> Result<Vec<SshHostMeta>, SshMetaRepoError> {
        if let Some(err) = self.list_error.lock().expect("lock").clone() {
            return Err(SshMetaRepoError::msg(err));
        }
        Ok(self.rows.lock().expect("lock").values().cloned().collect())
    }

    fn get(&self, alias: &str) -> Result<Option<SshHostMeta>, SshMetaRepoError> {
        Ok(self.rows.lock().expect("lock").get(alias).cloned())
    }

    fn set_favorite(&self, alias: &str, favorite: bool) -> Result<(), SshMetaRepoError> {
        let mut rows = self.rows.lock().expect("lock");
        let entry = rows
            .entry(alias.to_string())
            .or_insert_with(|| SshHostMeta {
                alias: alias.to_string(),
                display_name: None,
                favorite: false,
                tags: Vec::new(),
                last_connected_at: None,
                connection_count: 0,
            });
        entry.favorite = favorite;
        Ok(())
    }

    fn set_display_name(
        &self,
        alias: &str,
        display_name: Option<&str>,
    ) -> Result<(), SshMetaRepoError> {
        let mut rows = self.rows.lock().expect("lock");
        let entry = rows
            .entry(alias.to_string())
            .or_insert_with(|| SshHostMeta {
                alias: alias.to_string(),
                display_name: None,
                favorite: false,
                tags: Vec::new(),
                last_connected_at: None,
                connection_count: 0,
            });
        entry.display_name = display_name.map(str::to_string);
        Ok(())
    }

    fn record_connection(&self, alias: &str, connected_at: &str) -> Result<(), SshMetaRepoError> {
        let mut rows = self.rows.lock().expect("lock");
        let entry = rows
            .entry(alias.to_string())
            .or_insert_with(|| SshHostMeta {
                alias: alias.to_string(),
                display_name: None,
                favorite: false,
                tags: Vec::new(),
                last_connected_at: None,
                connection_count: 0,
            });
        entry.last_connected_at = Some(connected_at.to_string());
        entry.connection_count += 1;
        Ok(())
    }

    fn delete(&self, alias: &str) -> Result<(), SshMetaRepoError> {
        self.rows.lock().expect("lock").remove(alias);
        Ok(())
    }
}

/// In-memory timers for module tests.
#[derive(Default)]
pub struct MemoryTimersRepository {
    rows: Mutex<BTreeMap<String, TimerEntry>>,
    next: AtomicU64,
}

impl MemoryTimersRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TimersRepository for MemoryTimersRepository {
    fn list(&self) -> Result<Vec<TimerEntry>, TimersRepoError> {
        let mut rows: Vec<_> = self.rows.lock().expect("lock").values().cloned().collect();
        rows.sort_by(|a, b| {
            b.updated_at_ms
                .cmp(&a.updated_at_ms)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(rows)
    }

    fn get(&self, id: &str) -> Result<Option<TimerEntry>, TimersRepoError> {
        Ok(self.rows.lock().expect("lock").get(id).cloned())
    }

    fn insert(&self, entry: &TimerEntry) -> Result<(), TimersRepoError> {
        self.rows
            .lock()
            .expect("lock")
            .insert(entry.id.clone(), entry.clone());
        Ok(())
    }

    fn update(
        &self,
        entry: &TimerEntry,
        expected_updated_at_ms: i64,
    ) -> Result<(), TimersRepoError> {
        let mut rows = self.rows.lock().expect("lock");
        let Some(existing) = rows.get(&entry.id) else {
            return Err(TimersRepoError::Conflict);
        };
        if existing.updated_at_ms != expected_updated_at_ms {
            return Err(TimersRepoError::Conflict);
        }
        rows.insert(entry.id.clone(), entry.clone());
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), TimersRepoError> {
        self.rows.lock().expect("lock").remove(id);
        Ok(())
    }

    fn new_id(&self) -> String {
        let n = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        format!("tm-mem-{n}")
    }
}
