//! Notes SQLite index port (search / recent / scan maintenance).

use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct NotesSearchHit {
    pub relative_path: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesDocument {
    pub relative_path: String,
    pub title: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub mtime_unix: i64,
    pub tags: Vec<String>,
    pub outbound: Vec<NotesLink>,
    pub backlinks: Vec<NotesLink>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesLink {
    pub raw_href: String,
    pub target_path: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesIssue {
    pub relative_path: String,
    pub issue_type: String,
    pub message: String,
    pub scan_id: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesScanReport {
    pub mode: String,
    pub processed: usize,
    pub errors: usize,
    pub pruned: usize,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotesScanStatusView {
    Idle,
    Running {
        mode: String,
        processed: usize,
        total: usize,
    },
    Failed {
        message: String,
    },
    Completed {
        mode: String,
        processed: usize,
        total: usize,
        errors: usize,
    },
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct NotesIndexError(pub String);

impl NotesIndexError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[async_trait]
pub trait NotesIndexRepository: Send + Sync {
    fn search(&self, query: &str, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError>;
    fn list_recent(&self, limit: usize) -> Result<Vec<NotesSearchHit>, NotesIndexError>;
    fn document_count(&self) -> Result<usize, NotesIndexError>;
    fn fts_count(&self) -> Result<usize, NotesIndexError>;
    fn get_document(&self, relative_path: &str) -> Result<Option<NotesDocument>, NotesIndexError>;
    fn list_issues(&self) -> Result<Vec<NotesIssue>, NotesIndexError>;
    fn scan_status(&self) -> NotesScanStatusView;
    fn full_scan(
        &self,
        root: &Path,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError>;
    fn incremental_check(
        &self,
        root: &Path,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError>;
    fn rebuild(
        &self,
        root: &Path,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<NotesScanReport, NotesIndexError>;
}
