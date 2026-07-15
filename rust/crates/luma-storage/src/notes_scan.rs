//! Full and incremental notes workspace scanning into the SQLite index.

use crate::notes_discover::{discover, SkipReason, SkippedEntry};
use crate::notes_ignore::{rel_path_str, IgnoreMatcher};
use crate::notes_index_store::{
    DocumentLinkRow, DocumentRow, NotesIndexStore, NotesIndexStoreError, ScanIssueRow,
    ISSUE_FRONTMATTER_WARNING, ISSUE_OVERSIZED, ISSUE_SYMLINK_SKIPPED, ISSUE_UNREADABLE,
    ISSUE_WALK_ERROR,
};
use crate::notes_parse::{extract_body, extract_links, extract_tags, extract_title, LinkKind};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use thiserror::Error;

pub const DEFAULT_MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
const TAG_PREFIX_BYTES: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum NotesScanError {
    #[error(transparent)]
    Store(#[from] NotesIndexStoreError),
    #[error("scan already running")]
    Busy,
    #[error("scan cancelled")]
    Cancelled,
    #[error("incomplete discovery: {0}")]
    IncompleteDiscovery(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Default)]
pub struct NotesScanOptions {
    pub max_file_bytes: usize,
    pub ignore_dirs: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl NotesScanOptions {
    pub fn matcher(&self) -> IgnoreMatcher {
        IgnoreMatcher::new(self.ignore_dirs.clone(), self.exclude_patterns.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanMode {
    Full,
    Incremental,
    Rebuild,
}

#[derive(Clone, Debug)]
pub enum ScanStatus {
    Idle,
    Running {
        mode: ScanMode,
        processed: usize,
        total: usize,
    },
    Failed {
        message: String,
    },
    Completed {
        mode: ScanMode,
        processed: usize,
        total: usize,
        errors: usize,
    },
}

#[derive(Debug)]
pub struct ScanReport {
    pub mode: ScanMode,
    pub scan_id: i64,
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub pruned: usize,
    pub cancelled: bool,
}

pub struct NotesScanner {
    store: NotesIndexStore,
    busy: Mutex<bool>,
    status: Mutex<ScanStatus>,
    /// Test-only: relative paths that should fail reads (stable unreadable seam).
    force_unreadable: Mutex<HashSet<String>>,
}

impl NotesScanner {
    pub fn new(store: NotesIndexStore) -> Self {
        Self {
            store,
            busy: Mutex::new(false),
            status: Mutex::new(ScanStatus::Idle),
            force_unreadable: Mutex::new(HashSet::new()),
        }
    }

    /// Mark relative paths as unreadable for the next scan (tests only).
    pub fn force_unreadable_for_tests(&self, paths: impl IntoIterator<Item = impl Into<String>>) {
        *self.force_unreadable.lock().unwrap() = paths.into_iter().map(Into::into).collect();
    }

    pub fn store(&self) -> &NotesIndexStore {
        &self.store
    }

    pub fn status(&self) -> ScanStatus {
        self.status.lock().unwrap().clone()
    }

    fn try_begin(&self, mode: ScanMode, total: usize) -> Result<(), NotesScanError> {
        let mut busy = self.busy.lock().unwrap();
        if *busy {
            return Err(NotesScanError::Busy);
        }
        *busy = true;
        *self.status.lock().unwrap() = ScanStatus::Running {
            mode,
            processed: 0,
            total,
        };
        Ok(())
    }

    fn finish(&self, status: ScanStatus) {
        *self.busy.lock().unwrap() = false;
        *self.status.lock().unwrap() = status;
    }

    pub fn full_scan(
        &self,
        workspace_root: &Path,
        options: &NotesScanOptions,
        cancel: Option<&AtomicBool>,
    ) -> Result<ScanReport, NotesScanError> {
        self.run_scan(workspace_root, options, ScanMode::Full, cancel, false)
    }

    pub fn incremental_check(
        &self,
        workspace_root: &Path,
        options: &NotesScanOptions,
        cancel: Option<&AtomicBool>,
    ) -> Result<ScanReport, NotesScanError> {
        self.run_scan(
            workspace_root,
            options,
            ScanMode::Incremental,
            cancel,
            false,
        )
    }

    pub fn rebuild(
        &self,
        workspace_root: &Path,
        options: &NotesScanOptions,
        cancel: Option<&AtomicBool>,
    ) -> Result<ScanReport, NotesScanError> {
        self.run_scan(workspace_root, options, ScanMode::Rebuild, cancel, true)
    }

    fn run_scan(
        &self,
        workspace_root: &Path,
        options: &NotesScanOptions,
        mode: ScanMode,
        cancel: Option<&AtomicBool>,
        clear_first: bool,
    ) -> Result<ScanReport, NotesScanError> {
        if is_cancelled(cancel) {
            return Err(NotesScanError::Cancelled);
        }

        let matcher = options.matcher();
        let discovery = discover(workspace_root, &matcher, cancel);

        if discovery.cancelled {
            return Err(NotesScanError::Cancelled);
        }

        // Root-level discovery failure: do not touch the index.
        if !discovery.complete && discovery.files.is_empty() {
            let msg = discovery
                .skipped
                .iter()
                .find(|s| matches!(s.reason, SkipReason::WalkError))
                .map(|s| s.message.clone())
                .unwrap_or_else(|| "workspace discovery incomplete".into());
            return Err(NotesScanError::IncompleteDiscovery(msg));
        }

        let total = discovery.files.len();
        self.try_begin(mode.clone(), total)?;

        let result = (|| -> Result<ScanReport, NotesScanError> {
            // Cancel before destructive rebuild clear.
            if is_cancelled(cancel) {
                return Err(NotesScanError::Cancelled);
            }
            if clear_first {
                self.store.clear_all()?;
            }

            let scan_id = self.store.next_scan_id()?;
            let mut seen = Vec::with_capacity(discovery.files.len());
            let mut processed = 0usize;
            let mut errors = 0usize;
            let mut issues = Vec::new();
            let now = NotesIndexStore::now_unix();
            let scan_conn = self.store.connect()?;
            let mut scan_tx = scan_conn
                .unchecked_transaction()
                .map_err(NotesIndexStoreError::from)?;
            let mut batch_writes = 0usize;
            const SCAN_BATCH: usize = 32;

            for skip in &discovery.skipped {
                if let Some(issue) = skipped_to_issue(skip, scan_id, now) {
                    issues.push(issue);
                    errors += 1;
                }
            }

            for abs in &discovery.files {
                if is_cancelled(cancel) {
                    let _ = self.store.replace_issues_for_scan(scan_id, &issues);
                    return Err(NotesScanError::Cancelled);
                }

                let rel = rel_path_str(workspace_root, abs).ok_or_else(|| {
                    NotesScanError::Io(std::io::Error::other("path not under root"))
                })?;
                seen.push(rel.clone());

                match self.index_file(
                    abs,
                    &rel,
                    options,
                    scan_id,
                    mode == ScanMode::Incremental,
                    now,
                    Some(&mut scan_tx),
                ) {
                    Ok(file_issues) => {
                        errors += file_issues.len();
                        issues.extend(file_issues);
                    }
                    Err(e) => {
                        errors += 1;
                        issues.push(ScanIssueRow {
                            scan_id,
                            relative_path: rel,
                            issue_type: ISSUE_UNREADABLE.into(),
                            message: e.to_string(),
                            created_at_unix: now,
                        });
                    }
                }

                batch_writes += 1;
                if batch_writes >= SCAN_BATCH {
                    scan_tx.commit().map_err(NotesIndexStoreError::from)?;
                    scan_tx = scan_conn
                        .unchecked_transaction()
                        .map_err(NotesIndexStoreError::from)?;
                    batch_writes = 0;
                }

                processed += 1;
                {
                    let mut status = self.status.lock().unwrap();
                    if let ScanStatus::Running { .. } = *status {
                        *status = ScanStatus::Running {
                            mode: mode.clone(),
                            processed,
                            total,
                        };
                    }
                }
            }

            if batch_writes > 0 {
                scan_tx.commit().map_err(NotesIndexStoreError::from)?;
            }

            // Only prune after an authoritative complete enumeration.
            let pruned = if discovery.complete {
                self.store.prune_except(&seen)?
            } else {
                0
            };
            self.store.replace_issues_for_scan(scan_id, &issues)?;

            Ok(ScanReport {
                mode: mode.clone(),
                scan_id,
                processed,
                skipped: discovery.skipped.len(),
                errors,
                pruned,
                cancelled: false,
            })
        })();

        match &result {
            Ok(report) => {
                self.finish(ScanStatus::Completed {
                    mode: report.mode.clone(),
                    processed: report.processed,
                    total,
                    errors: report.errors,
                });
            }
            Err(NotesScanError::Cancelled) => {
                let processed = match &*self.status.lock().unwrap() {
                    ScanStatus::Running { processed, .. } => *processed,
                    _ => 0,
                };
                self.finish(ScanStatus::Failed {
                    message: format!("cancelled after {processed}/{total}"),
                });
            }
            Err(e) => {
                self.finish(ScanStatus::Failed {
                    message: e.to_string(),
                });
            }
        }

        result
    }

    #[allow(clippy::too_many_arguments)]
    fn index_file(
        &self,
        abs: &Path,
        rel: &str,
        options: &NotesScanOptions,
        scan_id: i64,
        incremental: bool,
        now: i64,
        batch_tx: Option<&mut rusqlite::Transaction<'_>>,
    ) -> Result<Vec<ScanIssueRow>, NotesScanError> {
        let meta = std::fs::symlink_metadata(abs)?;
        if meta.file_type().is_symlink() {
            return Ok(vec![ScanIssueRow {
                scan_id,
                relative_path: rel.to_string(),
                issue_type: ISSUE_SYMLINK_SKIPPED.into(),
                message: "symlink refused at index time".into(),
                created_at_unix: now,
            }]);
        }

        let size = meta.len() as i64;
        let mtime = file_mtime_unix(&meta);
        let file_name = abs
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("note.md")
            .to_string();

        if incremental {
            if let Some(existing) = self.store.get_document(rel)? {
                if existing.size_bytes == size && existing.mtime_unix == mtime {
                    return Ok(Vec::new());
                }
            }
        }

        let mut issues = Vec::new();

        if self.force_unreadable.lock().unwrap().contains(rel) {
            issues.push(ScanIssueRow {
                scan_id,
                relative_path: rel.to_string(),
                issue_type: ISSUE_UNREADABLE.into(),
                message: "forced unreadable (test)".into(),
                created_at_unix: now,
            });
            return Ok(issues);
        }

        if size as usize > options.max_file_bytes {
            let prefix = match read_nofollow_prefix(abs, TAG_PREFIX_BYTES) {
                Ok(p) => p,
                Err(e) => {
                    issues.push(ScanIssueRow {
                        scan_id,
                        relative_path: rel.to_string(),
                        issue_type: ISSUE_UNREADABLE.into(),
                        message: e.to_string(),
                        created_at_unix: now,
                    });
                    return Ok(issues);
                }
            };
            let tags = extract_tags(&prefix);
            let title = extract_title(&prefix, &file_name);
            let hash = content_hash(&prefix);

            self.upsert_indexed(
                &DocumentRow {
                    relative_path: rel.to_string(),
                    title: title.title.clone(),
                    file_name: file_name.clone(),
                    size_bytes: size,
                    mtime_unix: mtime,
                    content_hash: Some(hash),
                    updated_at_unix: now,
                },
                &tags.tags,
                &[],
                "",
                batch_tx,
            )?;

            issues.push(ScanIssueRow {
                scan_id,
                relative_path: rel.to_string(),
                issue_type: ISSUE_OVERSIZED.into(),
                message: format!("file exceeds {} bytes", options.max_file_bytes),
                created_at_unix: now,
            });
            return Ok(issues);
        }

        let content = match read_nofollow(abs) {
            Ok(c) => c,
            Err(e) => {
                issues.push(ScanIssueRow {
                    scan_id,
                    relative_path: rel.to_string(),
                    issue_type: ISSUE_UNREADABLE.into(),
                    message: e.to_string(),
                    created_at_unix: now,
                });
                return Ok(issues);
            }
        };

        let fm_warning = crate::notes_parse::split_frontmatter(&content).warning;
        let tags = extract_tags(&content);
        let title = extract_title(&content, &file_name);
        let body = extract_body(&content);
        let links = extract_links(&content, rel);
        let hash = content_hash(&content);

        let link_rows: Vec<DocumentLinkRow> = links
            .iter()
            .map(|link| {
                let kind = match link.kind {
                    LinkKind::Internal => "internal",
                    LinkKind::External => "external",
                };
                DocumentLinkRow {
                    source_path: rel.to_string(),
                    target_path: link.target_path.clone().unwrap_or_default(),
                    raw_href: link.raw_href.clone(),
                    kind: kind.into(),
                }
            })
            .collect();

        self.upsert_indexed(
            &DocumentRow {
                relative_path: rel.to_string(),
                title: title.title.clone(),
                file_name: file_name.clone(),
                size_bytes: size,
                mtime_unix: mtime,
                content_hash: Some(hash),
                updated_at_unix: now,
            },
            &tags.tags,
            &link_rows,
            &body,
            batch_tx,
        )?;

        if let Some(w) = fm_warning.or(tags.warning) {
            issues.push(ScanIssueRow {
                scan_id,
                relative_path: rel.to_string(),
                issue_type: ISSUE_FRONTMATTER_WARNING.into(),
                message: w,
                created_at_unix: now,
            });
        }

        Ok(issues)
    }

    fn upsert_indexed(
        &self,
        doc: &DocumentRow,
        tags: &[String],
        links: &[DocumentLinkRow],
        fts_body: &str,
        batch_tx: Option<&mut rusqlite::Transaction<'_>>,
    ) -> Result<(), NotesScanError> {
        match batch_tx {
            Some(tx) => NotesIndexStore::upsert_parsed_tx(tx, doc, tags, links, fts_body)
                .map_err(NotesScanError::Store),
            None => self
                .store
                .upsert_parsed(doc, tags, links, fts_body)
                .map_err(NotesScanError::Store),
        }
    }
}

fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|c| c.load(Ordering::Relaxed))
}

fn skipped_to_issue(skip: &SkippedEntry, scan_id: i64, now: i64) -> Option<ScanIssueRow> {
    let rel = skip.relative_path.clone()?;
    let issue_type = match skip.reason {
        SkipReason::SkipSymlink => ISSUE_SYMLINK_SKIPPED,
        SkipReason::WalkError => ISSUE_WALK_ERROR,
        _ => return None,
    };
    Some(ScanIssueRow {
        scan_id,
        relative_path: rel,
        issue_type: issue_type.into(),
        message: skip.message.clone(),
        created_at_unix: now,
    })
}

fn file_mtime_unix(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn read_nofollow(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    let mut file = open_nofollow(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_nofollow_prefix(path: &Path, max: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut file = open_nofollow(path)?;
    let mut buf = vec![0u8; max];
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

fn open_nofollow(path: &Path) -> Result<std::fs::File, std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Match notes.rs contained-create constants.
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(all(unix, not(target_os = "macos")))]
        const O_NOFOLLOW: i32 = 0o400000;

        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "refusing to follow symlink",
            ));
        }
        std::fs::File::open(path)
    }
}

fn content_hash(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes_index_store::NotesIndexStore;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use tempfile::TempDir;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/notes-workspaces")
            .join(name)
    }

    fn temp_scanner() -> (TempDir, NotesScanner) {
        let dir = TempDir::new().unwrap();
        let store = NotesIndexStore::with_path(dir.path().join("notes-index.sqlite")).unwrap();
        (dir, NotesScanner::new(store))
    }

    #[test]
    fn full_scan_basic_workspace() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("basic");
        let options = NotesScanOptions {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            ..Default::default()
        };
        let report = scanner.full_scan(&root, &options, None).unwrap();
        assert_eq!(report.processed, 7);
        assert_eq!(scanner.store().document_count().unwrap(), 7);
    }

    #[test]
    fn oversized_records_issue_and_empty_body() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("large");
        let options = NotesScanOptions {
            max_file_bytes: 512,
            ..Default::default()
        };
        let report = scanner.full_scan(&root, &options, None).unwrap();
        assert_eq!(report.processed, 2);
        let issues = scanner.store().list_issues(Some(report.scan_id)).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == ISSUE_OVERSIZED));
        let big = scanner.store().get_document("big.md").unwrap().unwrap();
        assert!(big.size_bytes > 512);
        assert_eq!(scanner.store().fts_count().unwrap(), 2);
    }

    #[test]
    fn incremental_skips_unchanged() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("basic");
        let options = NotesScanOptions::default();
        scanner.full_scan(&root, &options, None).unwrap();
        let report = scanner.incremental_check(&root, &options, None).unwrap();
        assert_eq!(report.processed, 7);
        assert_eq!(scanner.store().document_count().unwrap(), 7);
    }

    #[test]
    fn rebuild_clears_and_restores() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("basic");
        let options = NotesScanOptions::default();
        scanner.full_scan(&root, &options, None).unwrap();
        scanner.store().clear_all().unwrap();
        assert_eq!(scanner.store().document_count().unwrap(), 0);
        let report = scanner.rebuild(&root, &options, None).unwrap();
        assert_eq!(report.processed, 7);
        assert_eq!(scanner.store().document_count().unwrap(), 7);
    }

    #[test]
    fn busy_rejects_concurrent_scan() {
        let (_tmp, scanner) = temp_scanner();
        *scanner.busy.lock().unwrap() = true;
        let root = fixture_root("basic");
        let err = scanner
            .full_scan(&root, &NotesScanOptions::default(), None)
            .unwrap_err();
        assert!(matches!(err, NotesScanError::Busy));
    }

    #[test]
    fn unreadable_keeps_old_index_on_incremental() {
        let (tmp, scanner) = temp_scanner();
        let root = tmp.path().join("ws");
        fs::create_dir_all(&root).unwrap();
        let note = root.join("secret.md");
        fs::write(&note, "# Secret\n\ncontent").unwrap();

        let options = NotesScanOptions::default();
        scanner.full_scan(&root, &options, None).unwrap();
        let before = scanner.store().get_document("secret.md").unwrap().unwrap();

        // Stable seam: force a read failure without host ACL tricks.
        scanner.force_unreadable_for_tests(["secret.md"]);
        // Bump mtime so incremental re-reads.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let content = fs::read(&note).unwrap();
        fs::write(&note, content).unwrap();

        let report = scanner.incremental_check(&root, &options, None).unwrap();
        let after = scanner.store().get_document("secret.md").unwrap().unwrap();
        assert_eq!(before.title, after.title);
        assert_eq!(before.content_hash, after.content_hash);
        assert!(
            report.errors >= 1
                || scanner
                    .store()
                    .list_issues(None)
                    .unwrap()
                    .iter()
                    .any(|i| i.issue_type == ISSUE_UNREADABLE),
            "expected unreadable issue, report={report:?}"
        );
    }

    #[test]
    fn missing_root_does_not_prune_existing_index() {
        let (tmp, scanner) = temp_scanner();
        let root = tmp.path().join("ws");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.md"), "# Keep\n").unwrap();
        let options = NotesScanOptions::default();
        scanner.full_scan(&root, &options, None).unwrap();
        assert_eq!(scanner.store().document_count().unwrap(), 1);

        let missing = tmp.path().join("gone");
        let err = scanner.full_scan(&missing, &options, None).unwrap_err();
        assert!(matches!(err, NotesScanError::IncompleteDiscovery(_)));
        assert_eq!(scanner.store().document_count().unwrap(), 1);
        assert_eq!(scanner.store().fts_count().unwrap(), 1);
    }

    #[test]
    fn rebuild_pre_cancel_does_not_clear() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("basic");
        let options = NotesScanOptions::default();
        scanner.full_scan(&root, &options, None).unwrap();
        assert_eq!(scanner.store().document_count().unwrap(), 7);

        let cancel = AtomicBool::new(true);
        let err = scanner.rebuild(&root, &options, Some(&cancel)).unwrap_err();
        assert!(matches!(err, NotesScanError::Cancelled));
        assert_eq!(scanner.store().document_count().unwrap(), 7);
    }

    #[test]
    fn cancel_stops_scan() {
        let (_tmp, scanner) = temp_scanner();
        let root = fixture_root("basic");
        let cancel = AtomicBool::new(true);
        let err = scanner
            .full_scan(&root, &NotesScanOptions::default(), Some(&cancel))
            .unwrap_err();
        assert!(matches!(err, NotesScanError::Cancelled));
    }

    #[test]
    #[ignore = "phase5 real workspace removed — use tempdir fixtures only"]
    fn phase5_real_workspace_full_scan() {
        // Formerly deleted the real ~/Library/Application Support/LumaNext notes-index.sqlite.
        // Use `temp_scanner` + fixture trees in this crate for integration coverage instead.
    }
}
