use crate::ports::{
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit,
};
use async_trait::async_trait;
use luma_storage::{NotesScanOptions, NotesScanner, ScanMode, ScanStatus, DEFAULT_MAX_FILE_BYTES};
use std::path::Path;
use std::sync::Arc;

pub struct SqliteNotesIndex {
    scanner: Arc<NotesScanner>,
}

impl SqliteNotesIndex {
    pub fn new(scanner: Arc<NotesScanner>) -> Self {
        Self { scanner }
    }

    fn options() -> NotesScanOptions {
        NotesScanOptions {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            ..Default::default()
        }
    }

    fn map_report(r: luma_storage::ScanReport) -> NotesScanReport {
        NotesScanReport {
            mode: match r.mode {
                ScanMode::Full => "full".into(),
                ScanMode::Incremental => "incremental".into(),
                ScanMode::Rebuild => "rebuild".into(),
            },
            processed: r.processed,
            errors: r.errors,
            pruned: r.pruned,
            cancelled: r.cancelled,
        }
    }

    fn map_status(s: ScanStatus) -> NotesScanStatusView {
        match s {
            ScanStatus::Idle => NotesScanStatusView::Idle,
            ScanStatus::Running {
                mode,
                processed,
                total,
            } => NotesScanStatusView::Running {
                mode: match mode {
                    ScanMode::Full => "full".into(),
                    ScanMode::Incremental => "incremental".into(),
                    ScanMode::Rebuild => "rebuild".into(),
                },
                processed,
                total,
            },
            ScanStatus::Failed { message } => NotesScanStatusView::Failed { message },
            ScanStatus::Completed {
                mode,
                processed,
                total,
                errors,
            } => NotesScanStatusView::Completed {
                mode: match mode {
                    ScanMode::Full => "full".into(),
                    ScanMode::Incremental => "incremental".into(),
                    ScanMode::Rebuild => "rebuild".into(),
                },
                processed,
                total,
                errors,
            },
        }
    }
}

#[async_trait]
impl NotesIndexRepository for SqliteNotesIndex {
    fn search(&self, query: &str, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError> {
        self.scanner
            .store()
            .search(query, limit)
            .map(|hits| {
                hits.into_iter()
                    .map(|h| NotesSearchHit {
                        relative_path: h.relative_path,
                        title: h.title,
                        snippet: h.snippet,
                        rank: h.rank,
                    })
                    .collect()
            })
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError> {
        self.scanner
            .store()
            .list_recent(limit)
            .map(|docs| {
                docs.into_iter()
                    .map(|d| NotesSearchHit {
                        relative_path: d.relative_path.clone(),
                        title: d.title,
                        snippet: d.relative_path,
                        rank: 0.0,
                    })
                    .collect()
            })
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn document_count(&self) -> Result<usize, NotesIndexError> {
        self.scanner
            .store()
            .document_count()
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn fts_count(&self) -> Result<usize, NotesIndexError> {
        self.scanner
            .store()
            .fts_count()
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn get_document(&self, relative_path: &str) -> Result<Option<NotesDocument>, NotesIndexError> {
        let store = self.scanner.store();
        let Some(doc) = store
            .get_document(relative_path)
            .map_err(|e| NotesIndexError::msg(e.to_string()))?
        else {
            return Ok(None);
        };
        let tags = store
            .list_tags(relative_path)
            .map_err(|e| NotesIndexError::msg(e.to_string()))?;
        let outbound = store
            .list_outbound(relative_path)
            .map_err(|e| NotesIndexError::msg(e.to_string()))?
            .into_iter()
            .map(|l| NotesLink {
                raw_href: l.raw_href,
                target_path: l.target_path,
                kind: l.kind,
            })
            .collect();
        let backlinks = store
            .list_backlinks(relative_path)
            .map_err(|e| NotesIndexError::msg(e.to_string()))?
            .into_iter()
            .map(|l| NotesLink {
                raw_href: l.raw_href,
                target_path: l.source_path,
                kind: l.kind,
            })
            .collect();
        Ok(Some(NotesDocument {
            relative_path: doc.relative_path,
            title: doc.title,
            file_name: doc.file_name,
            size_bytes: doc.size_bytes,
            mtime_unix: doc.mtime_unix,
            tags,
            outbound,
            backlinks,
        }))
    }

    fn list_issues(&self) -> Result<Vec<NotesIssue>, NotesIndexError> {
        self.scanner
            .store()
            .list_issues(None)
            .map(|rows| {
                rows.into_iter()
                    .map(|r| NotesIssue {
                        relative_path: r.relative_path,
                        issue_type: r.issue_type,
                        message: r.message,
                        scan_id: r.scan_id,
                    })
                    .collect()
            })
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn scan_status(&self) -> NotesScanStatusView {
        Self::map_status(self.scanner.status())
    }

    fn full_scan(
        &self,
        root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        self.scanner
            .full_scan(root, &Self::options(), cancel.as_deref())
            .map(Self::map_report)
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn incremental_check(
        &self,
        root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        self.scanner
            .incremental_check(root, &Self::options(), cancel.as_deref())
            .map(Self::map_report)
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }

    fn rebuild(
        &self,
        root: &Path,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError> {
        self.scanner
            .rebuild(root, &Self::options(), cancel.as_deref())
            .map(Self::map_report)
            .map_err(|e| NotesIndexError::msg(e.to_string()))
    }
}
