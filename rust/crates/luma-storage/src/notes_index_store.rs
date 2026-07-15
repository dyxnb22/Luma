//! SQLite-backed rebuildable notes index (FTS5 + tags + links).

use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

pub const ISSUE_UNREADABLE: &str = "unreadable";
pub const ISSUE_OVERSIZED: &str = "oversized";
pub const ISSUE_FRONTMATTER_WARNING: &str = "frontmatter_warning";
pub const ISSUE_SYMLINK_SKIPPED: &str = "symlink_skipped";
pub const ISSUE_WALK_ERROR: &str = "walk_error";

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum NotesIndexStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("fts5 unavailable: {0}")]
    FtsUnavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentRow {
    pub relative_path: String,
    pub title: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub mtime_unix: i64,
    pub content_hash: Option<String>,
    pub updated_at_unix: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanIssueRow {
    pub scan_id: i64,
    pub relative_path: String,
    pub issue_type: String,
    pub message: String,
    pub created_at_unix: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentLinkRow {
    pub source_path: String,
    pub target_path: String,
    pub raw_href: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub relative_path: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

pub struct NotesIndexStore {
    path: PathBuf,
}

impl NotesIndexStore {
    pub fn luma_next_default() -> Result<Self, NotesIndexStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("notes-index.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, NotesIndexStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub(crate) fn connect(&self) -> Result<Connection, NotesIndexStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), NotesIndexStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes_schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS documents (
                relative_path TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                file_name TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                mtime_unix INTEGER NOT NULL,
                content_hash TEXT,
                updated_at_unix INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS scan_issues (
                scan_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                issue_type TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                PRIMARY KEY (scan_id, relative_path, issue_type)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                relative_path UNINDEXED,
                title,
                path,
                file_name,
                tags,
                body
            );

            CREATE TABLE IF NOT EXISTS document_tags (
                relative_path TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (relative_path, tag)
            );

            CREATE TABLE IF NOT EXISTS document_links (
                source_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                raw_href TEXT NOT NULL,
                kind TEXT NOT NULL,
                PRIMARY KEY (source_path, raw_href, kind, target_path)
            );

            CREATE TABLE IF NOT EXISTS scan_seq (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                last_scan_id INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO scan_seq(id, last_scan_id) VALUES (1, 0);
            "#,
        )?;

        // Single-row semantics: seed if empty, then drop any duplicate version rows.
        let version_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM notes_schema_version", [], |r| {
                r.get(0)
            })?;
        if version_count == 0 {
            conn.execute(
                "INSERT INTO notes_schema_version(version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        } else if version_count > 1 {
            conn.execute(
                "DELETE FROM notes_schema_version WHERE rowid NOT IN (
                    SELECT MIN(rowid) FROM notes_schema_version
                 )",
                [],
            )?;
        }
        let version: i64 = conn.query_row(
            "SELECT version FROM notes_schema_version LIMIT 1",
            [],
            |r| r.get(0),
        )?;
        if version != SCHEMA_VERSION {
            return Err(NotesIndexStoreError::FtsUnavailable(format!(
                "unsupported notes schema version {version}"
            )));
        }

        Self::verify_fts5(&conn)?;
        Ok(())
    }

    fn verify_fts5(conn: &Connection) -> Result<(), NotesIndexStoreError> {
        conn.execute_batch(
            r#"
            DROP TABLE IF EXISTS _fts5_self_check;
            CREATE VIRTUAL TABLE _fts5_self_check USING fts5(x);
            INSERT INTO _fts5_self_check(x) VALUES ('ok');
            "#,
        )
        .map_err(|e| NotesIndexStoreError::FtsUnavailable(e.to_string()))?;

        let ok: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _fts5_self_check WHERE _fts5_self_check MATCH 'ok'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| NotesIndexStoreError::FtsUnavailable(e.to_string()))?;

        conn.execute_batch("DROP TABLE IF EXISTS _fts5_self_check;")
            .map_err(|e| NotesIndexStoreError::FtsUnavailable(e.to_string()))?;

        if ok != 1 {
            return Err(NotesIndexStoreError::FtsUnavailable(
                "FTS5 MATCH self-check failed".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn now_unix() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    pub fn upsert_document(&self, doc: &DocumentRow) -> Result<(), NotesIndexStoreError> {
        self.connect()?.execute(
            r#"
            INSERT INTO documents (
                relative_path, title, file_name, size_bytes, mtime_unix, content_hash, updated_at_unix
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(relative_path) DO UPDATE SET
                title = excluded.title,
                file_name = excluded.file_name,
                size_bytes = excluded.size_bytes,
                mtime_unix = excluded.mtime_unix,
                content_hash = excluded.content_hash,
                updated_at_unix = excluded.updated_at_unix
            "#,
            params![
                doc.relative_path,
                doc.title,
                doc.file_name,
                doc.size_bytes,
                doc.mtime_unix,
                doc.content_hash,
                doc.updated_at_unix,
            ],
        )?;
        Ok(())
    }

    pub fn get_document(
        &self,
        relative_path: &str,
    ) -> Result<Option<DocumentRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT relative_path, title, file_name, size_bytes, mtime_unix, content_hash, updated_at_unix
             FROM documents WHERE relative_path = ?1",
        )?;
        let mut rows = stmt.query_map(params![relative_path], |row| {
            Ok(DocumentRow {
                relative_path: row.get(0)?,
                title: row.get(1)?,
                file_name: row.get(2)?,
                size_bytes: row.get(3)?,
                mtime_unix: row.get(4)?,
                content_hash: row.get(5)?,
                updated_at_unix: row.get(6)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn list_documents(&self) -> Result<Vec<DocumentRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT relative_path, title, file_name, size_bytes, mtime_unix, content_hash, updated_at_unix
             FROM documents ORDER BY relative_path",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DocumentRow {
                    relative_path: row.get(0)?,
                    title: row.get(1)?,
                    file_name: row.get(2)?,
                    size_bytes: row.get(3)?,
                    mtime_unix: row.get(4)?,
                    content_hash: row.get(5)?,
                    updated_at_unix: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn document_count(&self) -> Result<usize, NotesIndexStoreError> {
        let n: i64 = self
            .connect()?
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn fts_count(&self) -> Result<usize, NotesIndexStoreError> {
        let n: i64 = self
            .connect()?
            .query_row("SELECT COUNT(*) FROM documents_fts", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn replace_tags(
        &self,
        relative_path: &str,
        tags: &[String],
    ) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM document_tags WHERE relative_path = ?1",
            params![relative_path],
        )?;
        for tag in tags {
            conn.execute(
                "INSERT INTO document_tags (relative_path, tag) VALUES (?1, ?2)",
                params![relative_path, tag],
            )?;
        }
        Ok(())
    }

    pub fn list_tags(&self, relative_path: &str) -> Result<Vec<String>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT tag FROM document_tags WHERE relative_path = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map(params![relative_path], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    pub fn replace_links(
        &self,
        source_path: &str,
        links: &[DocumentLinkRow],
    ) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM document_links WHERE source_path = ?1",
            params![source_path],
        )?;
        for link in links {
            conn.execute(
                "INSERT INTO document_links (source_path, target_path, raw_href, kind)
                 VALUES (?1, ?2, ?3, ?4)",
                params![link.source_path, link.target_path, link.raw_href, link.kind],
            )?;
        }
        Ok(())
    }

    pub fn list_outbound(
        &self,
        source_path: &str,
    ) -> Result<Vec<DocumentLinkRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT source_path, target_path, raw_href, kind FROM document_links
             WHERE source_path = ?1 ORDER BY raw_href",
        )?;
        let rows = stmt
            .query_map(params![source_path], |row| {
                Ok(DocumentLinkRow {
                    source_path: row.get(0)?,
                    target_path: row.get(1)?,
                    raw_href: row.get(2)?,
                    kind: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_backlinks(
        &self,
        target_path: &str,
    ) -> Result<Vec<DocumentLinkRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT source_path, target_path, raw_href, kind FROM document_links
             WHERE target_path = ?1 AND kind = 'internal' ORDER BY source_path",
        )?;
        let rows = stmt
            .query_map(params![target_path], |row| {
                Ok(DocumentLinkRow {
                    source_path: row.get(0)?,
                    target_path: row.get(1)?,
                    raw_href: row.get(2)?,
                    kind: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Atomically upsert document metadata, tags, links, and FTS row.
    pub fn upsert_parsed(
        &self,
        doc: &DocumentRow,
        tags: &[String],
        links: &[DocumentLinkRow],
        fts_body: &str,
    ) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        Self::upsert_parsed_tx(&tx, doc, tags, links, fts_body)?;
        tx.commit()?;
        Ok(())
    }

    /// Upsert within an existing scan batch transaction.
    pub(crate) fn upsert_parsed_tx(
        tx: &rusqlite::Transaction<'_>,
        doc: &DocumentRow,
        tags: &[String],
        links: &[DocumentLinkRow],
        fts_body: &str,
    ) -> Result<(), NotesIndexStoreError> {
        let tags_joined = tags.join(" ");

        tx.execute(
            r#"
            INSERT INTO documents (
                relative_path, title, file_name, size_bytes, mtime_unix, content_hash, updated_at_unix
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(relative_path) DO UPDATE SET
                title = excluded.title,
                file_name = excluded.file_name,
                size_bytes = excluded.size_bytes,
                mtime_unix = excluded.mtime_unix,
                content_hash = excluded.content_hash,
                updated_at_unix = excluded.updated_at_unix
            "#,
            params![
                doc.relative_path,
                doc.title,
                doc.file_name,
                doc.size_bytes,
                doc.mtime_unix,
                doc.content_hash,
                doc.updated_at_unix,
            ],
        )?;

        tx.execute(
            "DELETE FROM document_tags WHERE relative_path = ?1",
            params![doc.relative_path],
        )?;
        for tag in tags {
            tx.execute(
                "INSERT INTO document_tags (relative_path, tag) VALUES (?1, ?2)",
                params![doc.relative_path, tag],
            )?;
        }

        tx.execute(
            "DELETE FROM document_links WHERE source_path = ?1",
            params![doc.relative_path],
        )?;
        for link in links {
            tx.execute(
                "INSERT INTO document_links (source_path, target_path, raw_href, kind)
                 VALUES (?1, ?2, ?3, ?4)",
                params![link.source_path, link.target_path, link.raw_href, link.kind],
            )?;
        }

        tx.execute(
            "DELETE FROM documents_fts WHERE relative_path = ?1",
            params![doc.relative_path],
        )?;
        tx.execute(
            "INSERT INTO documents_fts (relative_path, title, path, file_name, tags, body)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                doc.relative_path,
                doc.title,
                doc.relative_path,
                doc.file_name,
                tags_joined,
                fts_body,
            ],
        )?;

        Ok(())
    }

    pub fn upsert_fts(
        &self,
        relative_path: &str,
        title: &str,
        file_name: &str,
        tags: &str,
        body: &str,
    ) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM documents_fts WHERE relative_path = ?1",
            params![relative_path],
        )?;
        tx.execute(
            "INSERT INTO documents_fts (relative_path, title, path, file_name, tags, body)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![relative_path, title, relative_path, file_name, tags, body],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_document(&self, relative_path: &str) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM documents WHERE relative_path = ?1",
            params![relative_path],
        )?;
        tx.execute(
            "DELETE FROM documents_fts WHERE relative_path = ?1",
            params![relative_path],
        )?;
        tx.execute(
            "DELETE FROM document_tags WHERE relative_path = ?1",
            params![relative_path],
        )?;
        tx.execute(
            "DELETE FROM document_links WHERE source_path = ?1",
            params![relative_path],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            DELETE FROM documents;
            DELETE FROM documents_fts;
            DELETE FROM document_tags;
            DELETE FROM document_links;
            DELETE FROM scan_issues;
            "#,
        )?;
        Ok(())
    }

    pub fn next_scan_id(&self) -> Result<i64, NotesIndexStoreError> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE scan_seq SET last_scan_id = last_scan_id + 1 WHERE id = 1",
            [],
        )?;
        let id: i64 =
            conn.query_row("SELECT last_scan_id FROM scan_seq WHERE id = 1", [], |r| {
                r.get(0)
            })?;
        Ok(id)
    }

    pub fn replace_issues_for_scan(
        &self,
        scan_id: i64,
        issues: &[ScanIssueRow],
    ) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        // Latest-scan semantics: wipe prior issues, then insert this scan's set.
        tx.execute("DELETE FROM scan_issues", [])?;
        for issue in issues {
            tx.execute(
                "INSERT INTO scan_issues
                 (scan_id, relative_path, issue_type, message, created_at_unix)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    scan_id,
                    issue.relative_path,
                    issue.issue_type,
                    issue.message,
                    issue.created_at_unix,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn record_issue(&self, issue: &ScanIssueRow) -> Result<(), NotesIndexStoreError> {
        self.connect()?.execute(
            "INSERT OR REPLACE INTO scan_issues
             (scan_id, relative_path, issue_type, message, created_at_unix)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                issue.scan_id,
                issue.relative_path,
                issue.issue_type,
                issue.message,
                issue.created_at_unix,
            ],
        )?;
        Ok(())
    }

    pub fn list_issues(
        &self,
        scan_id: Option<i64>,
    ) -> Result<Vec<ScanIssueRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        if let Some(id) = scan_id {
            let mut stmt = conn.prepare(
                "SELECT scan_id, relative_path, issue_type, message, created_at_unix
                 FROM scan_issues WHERE scan_id = ?1 ORDER BY relative_path",
            )?;
            let rows = stmt
                .query_map(params![id], map_issue_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        } else {
            let mut stmt = conn.prepare(
                "SELECT scan_id, relative_path, issue_type, message, created_at_unix
                 FROM scan_issues ORDER BY scan_id DESC, relative_path",
            )?;
            let rows = stmt
                .query_map([], map_issue_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        }
    }

    pub fn clear_issues_for_paths(&self, paths: &[&str]) -> Result<(), NotesIndexStoreError> {
        let conn = self.connect()?;
        for path in paths {
            conn.execute(
                "DELETE FROM scan_issues WHERE relative_path = ?1",
                params![path],
            )?;
        }
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<DocumentRow>, NotesIndexStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT relative_path, title, file_name, size_bytes, mtime_unix, content_hash, updated_at_unix
             FROM documents ORDER BY mtime_unix DESC, relative_path ASC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(DocumentRow {
                    relative_path: row.get(0)?,
                    title: row.get(1)?,
                    file_name: row.get(2)?,
                    size_bytes: row.get(3)?,
                    mtime_unix: row.get(4)?,
                    content_hash: row.get(5)?,
                    updated_at_unix: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, NotesIndexStoreError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut hits = self.search_fts(trimmed, limit)?;
        if contains_cjk(trimmed) {
            let like_hits = self.search_like_fallback(trimmed, limit)?;
            merge_search_hits(&mut hits, like_hits, limit);
        }
        Ok(hits)
    }

    fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, NotesIndexStoreError> {
        let fts_query = escape_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.connect()?;
        let sql = "SELECT relative_path, title,
                    COALESCE(
                        NULLIF(snippet(documents_fts, 5, '[', ']', '…', 12), ''),
                        NULLIF(snippet(documents_fts, 4, '[', ']', '…', 12), ''),
                        NULLIF(snippet(documents_fts, 1, '[', ']', '…', 12), ''),
                        title
                    ) AS snip,
                    bm25(documents_fts) AS rank
             FROM documents_fts
             WHERE documents_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Ok(SearchHit {
                    relative_path: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    rank: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn search_like_fallback(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, NotesIndexStoreError> {
        let pattern = format!("%{}%", escape_like_pattern(query));
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT relative_path, title,
                    CASE
                        WHEN body LIKE ?1 ESCAPE '\\' THEN substr(body, 1, 96)
                        WHEN title LIKE ?1 ESCAPE '\\' THEN title
                        WHEN tags LIKE ?1 ESCAPE '\\' THEN tags
                        WHEN path LIKE ?1 ESCAPE '\\' THEN path
                        WHEN file_name LIKE ?1 ESCAPE '\\' THEN file_name
                        ELSE title
                    END AS snip
             FROM documents_fts
             WHERE body LIKE ?1 ESCAPE '\\'
                OR title LIKE ?1 ESCAPE '\\'
                OR tags LIKE ?1 ESCAPE '\\'
                OR path LIKE ?1 ESCAPE '\\'
                OR file_name LIKE ?1 ESCAPE '\\'
             ORDER BY relative_path
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(SearchHit {
                    relative_path: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    rank: 0.0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn prune_except(&self, keep: &[String]) -> Result<usize, NotesIndexStoreError> {
        let existing = self.list_documents()?;
        let keep_set: std::collections::HashSet<&str> = keep.iter().map(|s| s.as_str()).collect();
        let stale: Vec<String> = existing
            .into_iter()
            .filter(|doc| !keep_set.contains(doc.relative_path.as_str()))
            .map(|doc| doc.relative_path)
            .collect();
        if stale.is_empty() {
            return Ok(0);
        }
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        for path in &stale {
            tx.execute(
                "DELETE FROM documents WHERE relative_path = ?1",
                params![path],
            )?;
            tx.execute(
                "DELETE FROM documents_fts WHERE relative_path = ?1",
                params![path],
            )?;
            tx.execute(
                "DELETE FROM document_tags WHERE relative_path = ?1",
                params![path],
            )?;
            tx.execute(
                "DELETE FROM document_links WHERE source_path = ?1",
                params![path],
            )?;
        }
        tx.commit()?;
        Ok(stale.len())
    }
}

fn map_issue_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanIssueRow> {
    Ok(ScanIssueRow {
        scan_id: row.get(0)?,
        relative_path: row.get(1)?,
        issue_type: row.get(2)?,
        message: row.get(3)?,
        created_at_unix: row.get(4)?,
    })
}

/// Quote whitespace-separated terms and join with AND for FTS5.
pub fn escape_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|term| {
            let escaped = term.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn escape_like_pattern(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// True when the query contains CJK / Hiragana / Katakana / Hangul.
pub fn contains_cjk(query: &str) -> bool {
    query.chars().any(|c| {
        let u = c as u32;
        (0x4E00..=0x9FFF).contains(&u)
            || (0x3400..=0x4DBF).contains(&u)
            || (0xF900..=0xFAFF).contains(&u)
            || (0x3000..=0x303F).contains(&u)
            || (0x3040..=0x309F).contains(&u) // Hiragana
            || (0x30A0..=0x30FF).contains(&u) // Katakana
            || (0xAC00..=0xD7AF).contains(&u) // Hangul syllables
            || (0x1100..=0x11FF).contains(&u) // Hangul Jamo
            || (0x3130..=0x318F).contains(&u) // Hangul Compatibility Jamo
    })
}

fn merge_search_hits(fts: &mut Vec<SearchHit>, like: Vec<SearchHit>, limit: usize) {
    let mut seen: std::collections::HashSet<String> =
        fts.iter().map(|h| h.relative_path.clone()).collect();
    for hit in like {
        if seen.insert(hit.relative_path.clone()) {
            fts.push(hit);
        }
        if fts.len() >= limit {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn fixture(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/notes-workspaces")
            .join(rel)
    }

    fn temp_store() -> (TempDir, NotesIndexStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("notes-index.sqlite");
        let store = NotesIndexStore::with_path(path).unwrap();
        (dir, store)
    }

    #[test]
    fn fts_insert_and_search_english() {
        let (_dir, store) = temp_store();
        let now = NotesIndexStore::now_unix();
        store
            .upsert_document(&DocumentRow {
                relative_path: "alpha.md".into(),
                title: "Touch".into(),
                file_name: "alpha.md".into(),
                size_bytes: 10,
                mtime_unix: now,
                content_hash: None,
                updated_at_unix: now,
            })
            .unwrap();
        store
            .upsert_fts("alpha.md", "Touch", "alpha.md", "", "touch heading content")
            .unwrap();
        let hits = store.search("touch", 10).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].relative_path, "alpha.md");
    }

    #[test]
    fn fts_search_cjk_phrase() {
        let (_dir, store) = temp_store();
        let content = fs::read_to_string(fixture("cjk/chinese-note.md")).unwrap();
        let now = NotesIndexStore::now_unix();
        store
            .upsert_document(&DocumentRow {
                relative_path: "chinese-note.md".into(),
                title: "中文短语检索样例".into(),
                file_name: "chinese-note.md".into(),
                size_bytes: content.len() as i64,
                mtime_unix: now,
                content_hash: None,
                updated_at_unix: now,
            })
            .unwrap();
        store
            .upsert_fts(
                "chinese-note.md",
                "中文短语检索样例",
                "chinese-note.md",
                "",
                &content,
            )
            .unwrap();
        let hits = store.search("知识工作区导航", 10).unwrap();
        assert!(
            hits.iter().any(|h| h.relative_path == "chinese-note.md"),
            "hits: {hits:?}"
        );
    }

    #[test]
    fn escape_fts_query_quotes_terms() {
        assert_eq!(escape_fts_query("foo bar"), "\"foo\" AND \"bar\"");
    }

    #[test]
    fn contains_cjk_detects_han() {
        assert!(contains_cjk("知识"));
        assert!(!contains_cjk("hello"));
        assert!(contains_cjk("あいう")); // Hiragana
        assert!(contains_cjk("カタカナ")); // Katakana
        assert!(contains_cjk("한글")); // Hangul syllables
        assert!(contains_cjk("\u{1100}")); // Hangul Jamo
        assert!(contains_cjk("\u{3131}")); // Compatibility Jamo
    }

    #[test]
    fn clear_all_empties_tables() {
        let (_dir, store) = temp_store();
        let now = NotesIndexStore::now_unix();
        store
            .upsert_document(&DocumentRow {
                relative_path: "x.md".into(),
                title: "X".into(),
                file_name: "x.md".into(),
                size_bytes: 1,
                mtime_unix: now,
                content_hash: None,
                updated_at_unix: now,
            })
            .unwrap();
        store.clear_all().unwrap();
        assert_eq!(store.document_count().unwrap(), 0);
    }
}
