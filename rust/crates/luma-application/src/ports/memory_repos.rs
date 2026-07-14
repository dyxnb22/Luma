use crate::ports::{
    ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError, NotesDocument, NotesIndexError,
    NotesIndexRepository, NotesIssue, NotesScanReport, NotesScanStatusView, NotesSearchHit,
    QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository, SnippetEntry, SnippetsRepoError,
    SnippetsRepository,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

#[derive(Clone, Debug)]
struct MemNote {
    relative_path: String,
    title: String,
    body: String,
    mtime_unix: i64,
    size_bytes: i64,
}

/// Lightweight in-memory notes index for module tests (no SQLite).
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

    fn scan_into(docs: &mut BTreeMap<String, MemNote>, root: &Path) {
        docs.clear();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    if path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with('.'))
                    {
                        continue;
                    }
                    stack.push(path);
                    continue;
                }
                let is_md = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("md"));
                if !is_md {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(root) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let title = content
                    .lines()
                    .find_map(|l| {
                        let t = l.trim();
                        t.strip_prefix('#').map(|rest| rest.trim().to_string())
                    })
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| {
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("note")
                            .to_string()
                    });
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                docs.insert(
                    rel.clone(),
                    MemNote {
                        relative_path: rel,
                        title,
                        body: content,
                        mtime_unix: mtime,
                        size_bytes: meta.len() as i64,
                    },
                );
            }
        }
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
        root: &Path,
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
        let mut docs = self.docs.lock().expect("lock");
        Self::scan_into(&mut docs, root);
        let n = docs.len();
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
