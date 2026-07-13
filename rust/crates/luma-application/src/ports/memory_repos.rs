use crate::ports::{
    ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, QuicklinkEntry,
    QuicklinksRepoError, QuicklinksRepository, SnippetEntry, SnippetsRepoError, SnippetsRepository,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
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
