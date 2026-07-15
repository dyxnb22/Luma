//! Records SQLite store (personal media/content log). Paths under LumaNext only.

use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use crate::records_parse::{
    hash_record_content, normalize_record_name, ParseFileReport, ParsedRecordRow,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 2;

const RECORD_COLS: &str = "r.id, r.category_id, r.category_name, r.name, r.name_normalized,
    r.rating, r.note, r.source_file, r.source_key, r.source_hash,
    r.created_at, r.updated_at";

#[derive(Debug, Error)]
pub enum RecordsStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCategoryRow {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub source_file: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRow {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    pub name_normalized: String,
    pub rating: Option<i64>,
    pub note: String,
    pub source_file: String,
    pub source_key: String,
    pub source_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecordsStats {
    pub categories: i64,
    pub records: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize)]
pub struct RecordImportApplyReport {
    pub categories_upserted: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize)]
pub struct RecordImportPreview {
    pub files_found: usize,
    pub categories: usize,
    pub records: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub per_file: Vec<ParseFileReport>,
}

pub fn preview_import_from_dir(root: &Path) -> Result<RecordImportPreview, RecordsStoreError> {
    let parsed = scan_records_dir(root)?;
    let mut preview = RecordImportPreview {
        files_found: parsed.len(),
        ..Default::default()
    };
    let mut category_names = std::collections::BTreeSet::new();
    for file in &parsed {
        preview.warnings.extend(file.warnings.clone());
        preview.errors.extend(file.errors.clone());
        if file.errors.is_empty() {
            category_names.insert(file.category_name.clone());
            preview.records += file.rows.len();
        }
        preview.per_file.push(file.clone());
    }
    preview.categories = category_names.len();
    Ok(preview)
}

pub struct RecordsStore {
    path: PathBuf,
}

impl RecordsStore {
    pub fn luma_next_default() -> Result<Self, RecordsStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("records.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, RecordsStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> Result<Connection, RecordsStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), RecordsStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS records_schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS record_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                sort_order INTEGER NOT NULL DEFAULT 0,
                source_file TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL REFERENCES record_categories(id),
                category_name TEXT NOT NULL,
                name TEXT NOT NULL,
                name_normalized TEXT NOT NULL,
                rating INTEGER CHECK (rating IS NULL OR (rating >= 1 AND rating <= 10)),
                note TEXT NOT NULL DEFAULT '',
                source_file TEXT NOT NULL DEFAULT '',
                source_key TEXT NOT NULL DEFAULT '',
                source_hash TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(category_id, name_normalized),
                UNIQUE(source_file, source_key)
            );

            CREATE TABLE IF NOT EXISTS record_tombstones (
                source_file TEXT NOT NULL,
                source_key TEXT NOT NULL,
                deleted_at TEXT NOT NULL,
                PRIMARY KEY(source_file, source_key)
            );

            CREATE INDEX IF NOT EXISTS idx_records_category ON records(category_id);
            CREATE INDEX IF NOT EXISTS idx_records_rating ON records(rating);
            CREATE INDEX IF NOT EXISTS idx_records_updated ON records(updated_at);
            "#,
        )?;

        let version_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM records_schema_version", [], |r| {
                r.get(0)
            })?;
        if version_count == 0 {
            conn.execute(
                "INSERT INTO records_schema_version(version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        }
        let version: i64 = conn.query_row(
            "SELECT version FROM records_schema_version LIMIT 1",
            [],
            |r| r.get(0),
        )?;
        if version > SCHEMA_VERSION {
            return Err(RecordsStoreError::Msg(format!(
                "unsupported records schema version {version}"
            )));
        }
        if version < SCHEMA_VERSION {
            conn.execute(
                "UPDATE records_schema_version SET version = ?1",
                params![SCHEMA_VERSION],
            )?;
        }
        ensure_records_fts(&conn)?;
        Ok(())
    }

    pub fn stats(&self) -> Result<RecordsStats, RecordsStoreError> {
        let conn = self.connect()?;
        let categories: i64 =
            conn.query_row("SELECT COUNT(*) FROM record_categories", [], |r| r.get(0))?;
        let records: i64 = conn.query_row("SELECT COUNT(*) FROM records", [], |r| r.get(0))?;
        Ok(RecordsStats {
            categories,
            records,
        })
    }

    pub fn list_categories(&self) -> Result<Vec<RecordCategoryRow>, RecordsStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, sort_order, source_file, created_at, updated_at
             FROM record_categories ORDER BY sort_order, name",
        )?;
        let rows = stmt
            .query_map([], map_category_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_category_by_name(
        &self,
        name: &str,
    ) -> Result<Option<RecordCategoryRow>, RecordsStoreError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, name, sort_order, source_file, created_at, updated_at
             FROM record_categories WHERE name = ?1",
            params![name],
            map_category_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get(&self, id: i64) -> Result<Option<RecordRow>, RecordsStoreError> {
        let conn = self.connect()?;
        conn.query_row(
            &format!("SELECT {RECORD_COLS} FROM records r WHERE r.id = ?1"),
            params![id],
            map_record_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn get_by_category_and_name(
        &self,
        category_name: &str,
        name: &str,
    ) -> Result<Option<RecordRow>, RecordsStoreError> {
        let norm = normalize_record_name(name);
        let conn = self.connect()?;
        conn.query_row(
            &format!(
                "SELECT {RECORD_COLS} FROM records r
                 JOIN record_categories c ON c.id = r.category_id
                 WHERE c.name = ?1 AND r.name_normalized = ?2"
            ),
            params![category_name, norm],
            map_record_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_by_category(
        &self,
        category_name: &str,
        limit: usize,
    ) -> Result<Vec<RecordRow>, RecordsStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {RECORD_COLS} FROM records r
             WHERE r.category_name = ?1
             ORDER BY r.rating IS NULL, r.rating DESC, r.updated_at DESC, r.name ASC
             LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![category_name, limit as i64], map_record_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn search(
        &self,
        query: &str,
        category_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordRow>, RecordsStoreError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(cat) = category_filter {
            return self.search_in_category(cat, q, limit);
        }
        let conn = self.connect()?;
        let fts_q = escape_fts_query(q);
        if !fts_q.is_empty() {
            if let Ok(rows) = self.search_fts(&conn, &fts_q, None, limit) {
                if !rows.is_empty() {
                    return Ok(rows);
                }
            }
        }
        self.search_like(&conn, q, None, limit)
    }

    fn search_in_category(
        &self,
        category: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RecordRow>, RecordsStoreError> {
        let conn = self.connect()?;
        let fts_q = escape_fts_query(query);
        if !fts_q.is_empty() {
            if let Ok(rows) = self.search_fts(&conn, &fts_q, Some(category), limit) {
                if !rows.is_empty() {
                    return Ok(rows);
                }
            }
        }
        self.search_like(&conn, query, Some(category), limit)
    }

    fn search_fts(
        &self,
        conn: &Connection,
        fts_q: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordRow>, RecordsStoreError> {
        if let Some(cat) = category {
            let mut stmt = conn.prepare(&format!(
                "SELECT {RECORD_COLS}
                 FROM records_fts f
                 JOIN records r ON r.id = f.rowid
                 WHERE records_fts MATCH ?1 AND r.category_name = ?2
                 ORDER BY bm25(records_fts), r.rating IS NULL, r.rating DESC
                 LIMIT ?3"
            ))?;
            let mapped = stmt.query_map(params![fts_q, cat, limit as i64], map_record_row)?;
            return mapped.collect::<Result<Vec<_>, _>>().map_err(Into::into);
        }
        let mut stmt = conn.prepare(&format!(
            "SELECT {RECORD_COLS}
             FROM records_fts f
             JOIN records r ON r.id = f.rowid
             WHERE records_fts MATCH ?1
             ORDER BY bm25(records_fts), r.rating IS NULL, r.rating DESC
             LIMIT ?2"
        ))?;
        let mapped = stmt.query_map(params![fts_q, limit as i64], map_record_row)?;
        mapped.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn search_like(
        &self,
        conn: &Connection,
        q: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordRow>, RecordsStoreError> {
        let pattern = format!("%{q}%");
        let rows = if let Some(cat) = category {
            let mut stmt = conn.prepare(&format!(
                "SELECT {RECORD_COLS} FROM records r
                 WHERE r.category_name = ?1
                   AND (r.name LIKE ?2 OR r.note LIKE ?2 OR r.category_name LIKE ?2)
                 ORDER BY r.rating IS NULL, r.rating DESC, r.updated_at DESC
                 LIMIT ?3"
            ))?;
            let mapped = stmt.query_map(params![cat, pattern, limit as i64], map_record_row)?;
            mapped.collect::<Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(&format!(
                "SELECT {RECORD_COLS} FROM records r
                 WHERE r.name LIKE ?1 OR r.note LIKE ?1 OR r.category_name LIKE ?1
                 ORDER BY r.rating IS NULL, r.rating DESC, r.updated_at DESC
                 LIMIT ?2"
            ))?;
            let mapped = stmt.query_map(params![pattern, limit as i64], map_record_row)?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn insert_record(
        &self,
        category_name: &str,
        name: &str,
        rating: Option<i64>,
        note: &str,
    ) -> Result<RecordRow, RecordsStoreError> {
        self.validate_rating(rating)?;
        let category_name = category_name.trim();
        if category_name.is_empty() {
            return Err(RecordsStoreError::Msg("category is empty".into()));
        }
        let norm = normalize_record_name(name);
        if norm.is_empty() {
            return Err(RecordsStoreError::Msg("name is empty".into()));
        }
        let conn = self.connect()?;
        let now = now_iso();
        let tx = conn.unchecked_transaction()?;
        let category_id: i64 = tx
            .query_row(
                "SELECT id FROM record_categories WHERE name = ?1",
                params![category_name.trim()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                RecordsStoreError::Msg(format!(
                    "category \"{}\" does not exist; import categories first",
                    category_name.trim()
                ))
            })?;
        tx.execute(
            "INSERT INTO records (
                category_id, category_name, name, name_normalized, rating, note,
                source_file, source_key, source_hash, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', '', '', ?7, ?7)",
            params![
                category_id,
                category_name.trim(),
                name.trim(),
                norm,
                rating,
                note,
                now
            ],
        )
        .map_err(|e| {
            if is_unique_violation(&e) {
                RecordsStoreError::Msg(format!(
                    "record already exists in category \"{category_name}\""
                ))
            } else {
                RecordsStoreError::Sqlite(e)
            }
        })?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| RecordsStoreError::Msg("insert failed".into()))
    }

    pub fn update_record(
        &self,
        id: i64,
        name: Option<&str>,
        rating: Option<Option<i64>>,
        note: Option<&str>,
        category_name: Option<&str>,
    ) -> Result<RecordRow, RecordsStoreError> {
        let Some(existing) = self.get(id)? else {
            return Err(RecordsStoreError::Msg(format!("record {id} not found")));
        };
        if let Some(Some(r)) = rating {
            self.validate_rating(Some(r))?;
        }
        let conn = self.connect()?;
        let now = now_iso();
        let tx = conn.unchecked_transaction()?;

        let mut category_id = existing.category_id;
        let mut category_name_str = existing.category_name.clone();
        if let Some(cat) = category_name {
            category_id = tx
                .query_row(
                    "SELECT id FROM record_categories WHERE name = ?1",
                    params![cat.trim()],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| {
                    RecordsStoreError::Msg(format!(
                        "category \"{}\" does not exist; import categories first",
                        cat.trim()
                    ))
                })?;
            category_name_str = cat.trim().to_string();
        }

        let new_name = name.unwrap_or(&existing.name);
        let norm = normalize_record_name(new_name);
        if norm.is_empty() {
            return Err(RecordsStoreError::Msg("name is empty".into()));
        }
        let new_rating = match rating {
            Some(r) => r,
            None => existing.rating,
        };
        let new_note = note.unwrap_or(&existing.note);

        tx.execute(
            "UPDATE records SET
                category_id = ?1, category_name = ?2, name = ?3, name_normalized = ?4,
                rating = ?5, note = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                category_id,
                category_name_str,
                new_name.trim(),
                norm,
                new_rating,
                new_note,
                now,
                id
            ],
        )
        .map_err(|e| {
            if is_unique_violation(&e) {
                RecordsStoreError::Msg("duplicate name in category".into())
            } else {
                RecordsStoreError::Sqlite(e)
            }
        })?;
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| RecordsStoreError::Msg("update failed".into()))
    }

    pub fn set_rating(&self, id: i64, rating: Option<i64>) -> Result<RecordRow, RecordsStoreError> {
        self.validate_rating(rating)?;
        self.update_record(id, None, Some(rating), None, None)
    }

    pub fn set_note(&self, id: i64, note: &str) -> Result<RecordRow, RecordsStoreError> {
        self.update_record(id, None, None, Some(note), None)
    }

    pub fn delete(&self, id: i64) -> Result<(), RecordsStoreError> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        let source: Option<(String, String)> = tx
            .query_row(
                "SELECT source_file, source_key FROM records WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((source_file, source_key)) = source else {
            return Err(RecordsStoreError::Msg(format!("record {id} not found")));
        };
        if !source_file.is_empty() && !source_key.is_empty() {
            tx.execute(
                "INSERT OR REPLACE INTO record_tombstones(source_file, source_key, deleted_at)
                 VALUES (?1, ?2, ?3)",
                params![source_file, source_key, now_iso()],
            )?;
        }
        tx.execute("DELETE FROM records WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn preview_import_dir(
        &self,
        root: &Path,
    ) -> Result<RecordImportPreview, RecordsStoreError> {
        preview_import_from_dir(root)
    }

    pub fn import_dir(&self, root: &Path) -> Result<RecordImportApplyReport, RecordsStoreError> {
        let parsed = scan_records_dir(root)?;
        let mut report = RecordImportApplyReport::default();
        let mut categories_seen = std::collections::BTreeSet::new();
        let conn = self.connect()?;
        let now = now_iso();
        let tx = conn.unchecked_transaction()?;
        for file in &parsed {
            report.warnings.extend(file.warnings.clone());
            if !file.errors.is_empty() {
                report.errors.extend(file.errors.clone());
                continue;
            }
            let category_id =
                upsert_category_tx(&tx, &file.category_name, &file.source_file, &now)?;
            if categories_seen.insert(file.category_name.clone()) {
                report.categories_upserted += 1;
            }
            for row in &file.rows {
                match import_row_tx(
                    &tx,
                    category_id,
                    &file.category_name,
                    row,
                    &file.source_file,
                    &now,
                ) {
                    Ok(true) => report.inserted += 1,
                    Ok(false) => report.skipped += 1,
                    Err(e) => report.warnings.push(e),
                }
            }
        }
        if report.errors.is_empty() {
            tx.commit()?;
        } else {
            return Err(RecordsStoreError::Msg(format!(
                "import aborted: {}",
                report.errors.join("; ")
            )));
        }
        Ok(report)
    }

    pub fn backup(&self) -> Result<PathBuf, RecordsStoreError> {
        ensure_luma_next_dirs()?;
        let support = luma_next_support_dir()?;
        let backups = support.join("backups");
        std::fs::create_dir_all(&backups)?;
        let now = Utc::now();
        let stamp = format!(
            "{}-{:03}",
            now.format("%Y%m%d-%H%M%S"),
            now.timestamp_subsec_millis()
        );
        let dest = backups.join(format!("records-backup-{stamp}.sqlite"));
        let tmp = backups.join(format!("records-backup-{stamp}.sqlite.tmp"));
        let _ = std::fs::remove_file(&tmp);
        let conn = self.connect()?;
        let quoted = sqlite_path_literal(&tmp)?;
        conn.execute_batch(&format!("VACUUM INTO {quoted}"))?;
        std::fs::rename(&tmp, &dest)?;
        Ok(dest)
    }

    fn validate_rating(&self, rating: Option<i64>) -> Result<(), RecordsStoreError> {
        if let Some(r) = rating {
            if !(1..=10).contains(&r) {
                return Err(RecordsStoreError::Msg(
                    "rating must be between 1 and 10".into(),
                ));
            }
        }
        Ok(())
    }
}

fn scan_records_dir(root: &Path) -> Result<Vec<ParseFileReport>, RecordsStoreError> {
    if !root.is_dir() {
        return Err(RecordsStoreError::Msg(format!(
            "not a directory: {}",
            root.display()
        )));
    }
    let canonical_root = root.canonicalize()?;
    let mut files = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(root)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            files.push(ParseFileReport {
                source_file: name.clone(),
                category_name: category_from_filename(&name),
                errors: vec![format!("{name}: symlink skipped")],
                ..Default::default()
            });
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                files.push(ParseFileReport {
                    source_file: name.clone(),
                    category_name: category_from_filename(&name),
                    errors: vec![format!("{name}: read failed: {e}")],
                    ..Default::default()
                });
                continue;
            }
        };
        let category = category_from_filename(&name);
        let source_identity = canonical_root.join(&name).to_string_lossy().to_string();
        files.push(crate::records_parse::parse_markdown_file_with_identity(
            &name,
            &source_identity,
            &category,
            &bytes,
        ));
    }
    Ok(files)
}

fn category_from_filename(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_string()
}

fn upsert_category_tx(
    tx: &Connection,
    name: &str,
    source_file: &str,
    now: &str,
) -> Result<i64, RecordsStoreError> {
    tx.execute(
        "INSERT INTO record_categories (name, sort_order, source_file, created_at, updated_at)
         VALUES (?1, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM record_categories), ?2, ?3, ?3)
         ON CONFLICT(name) DO UPDATE SET
            source_file = CASE WHEN excluded.source_file != '' THEN excluded.source_file ELSE record_categories.source_file END,
            updated_at = excluded.updated_at",
        params![name, source_file, now],
    )?;
    let id: i64 = tx.query_row(
        "SELECT id FROM record_categories WHERE name = ?1",
        params![name],
        |r| r.get(0),
    )?;
    Ok(id)
}

fn import_row_tx(
    tx: &Connection,
    category_id: i64,
    category_name: &str,
    row: &ParsedRecordRow,
    source_file: &str,
    now: &str,
) -> Result<bool, String> {
    let norm = normalize_record_name(&row.name);
    let row_hash = hash_record_content(row);

    let tombstoned: bool = tx
        .query_row(
            "SELECT 1 FROM record_tombstones WHERE source_file = ?1 AND source_key = ?2",
            params![source_file, row.source_key],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| {
            format!(
                "{source_file}:{}: tombstone lookup failed: {e}",
                row.line_no
            )
        })?
        .unwrap_or(false);
    if tombstoned {
        return Ok(false);
    }

    if let Ok(existing_hash) = tx.query_row::<String, _, _>(
        "SELECT source_hash FROM records WHERE source_file = ?1 AND source_key = ?2",
        params![source_file, row.source_key],
        |r| r.get(0),
    ) {
        if existing_hash == row_hash {
            return Ok(false);
        }
        return Err(format!(
            "{}:{}: source changed but DB row exists; skipped (DB wins)",
            source_file, row.line_no
        ));
    }

    if tx
        .query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM records WHERE category_id = ?1 AND name_normalized = ?2",
            params![category_id, norm],
            |r| r.get(0),
        )
        .map_err(|e| {
            format!(
                "{source_file}:{}: duplicate lookup failed: {e}",
                row.line_no
            )
        })?
        > 0
    {
        return Err(format!(
            "{}:{}: duplicate name \"{}\" in category; skipped",
            source_file, row.line_no, row.name
        ));
    }

    tx.execute(
        "INSERT INTO records (
            category_id, category_name, name, name_normalized, rating, note,
            source_file, source_key, source_hash, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            category_id,
            category_name,
            row.name.trim(),
            norm,
            row.rating,
            row.note,
            source_file,
            row.source_key,
            row_hash,
            now
        ],
    )
    .map_err(|e| format!("{source_file}:{}: insert failed: {e}", row.line_no))?;
    Ok(true)
}

fn map_category_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RecordCategoryRow> {
    Ok(RecordCategoryRow {
        id: r.get(0)?,
        name: r.get(1)?,
        sort_order: r.get(2)?,
        source_file: r.get(3)?,
        created_at: r.get(4)?,
        updated_at: r.get(5)?,
    })
}

fn map_record_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RecordRow> {
    Ok(RecordRow {
        id: r.get(0)?,
        category_id: r.get(1)?,
        category_name: r.get(2)?,
        name: r.get(3)?,
        name_normalized: r.get(4)?,
        rating: r.get(5)?,
        note: r.get(6)?,
        source_file: r.get(7)?,
        source_key: r.get(8)?,
        source_hash: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

fn ensure_records_fts(conn: &Connection) -> Result<(), RecordsStoreError> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'records_fts'",
            [],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if exists {
        return Ok(());
    }
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE records_fts USING fts5(
            name, note, category_name,
            content='records', content_rowid='id'
        );
        INSERT INTO records_fts(rowid, name, note, category_name)
          SELECT id, name, note, category_name FROM records;
        CREATE TRIGGER records_fts_ai AFTER INSERT ON records BEGIN
          INSERT INTO records_fts(rowid, name, note, category_name)
          VALUES (new.id, new.name, new.note, new.category_name);
        END;
        CREATE TRIGGER records_fts_ad AFTER DELETE ON records BEGIN
          INSERT INTO records_fts(records_fts, rowid, name, note, category_name)
          VALUES ('delete', old.id, old.name, old.note, old.category_name);
        END;
        CREATE TRIGGER records_fts_au AFTER UPDATE ON records BEGIN
          INSERT INTO records_fts(records_fts, rowid, name, note, category_name)
          VALUES ('delete', old.id, old.name, old.note, old.category_name);
          INSERT INTO records_fts(rowid, name, note, category_name)
          VALUES (new.id, new.name, new.note, new.category_name);
        END;
        "#,
    )?;
    Ok(())
}

fn escape_fts_query(q: &str) -> String {
    let tokens: Vec<String> = q
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let cleaned: String = t
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{cleaned}\"*")
            }
        })
        .filter(|t| !t.is_empty())
        .collect();
    tokens.join(" ")
}

fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(code, _)
            if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                || code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT
    )
}

fn sqlite_path_literal(path: &Path) -> Result<String, RecordsStoreError> {
    let s = path.to_string_lossy().replace('\'', "''");
    Ok(format!("'{s}'"))
}

pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RecordsImportLedgerReport {
    pub preview: RecordImportPreview,
    pub apply: Option<RecordImportApplyReport>,
    pub migration: Option<crate::migration_ledger::PersistedMigration>,
}

pub fn import_records_with_ledger(
    root: &Path,
    store: &RecordsStore,
    commit: bool,
) -> Result<RecordsImportLedgerReport, RecordsStoreError> {
    use crate::migration_ledger::{record_dry_run, MigrationCommitGuard, MigrationKind};
    let preview = preview_import_from_dir(root)?;
    if !commit {
        let migration = record_dry_run(
            MigrationKind::Records,
            root,
            preview.records as u64,
            0,
            preview.errors.len() as u64,
            preview.warnings.clone(),
        )
        .map_err(|e| RecordsStoreError::Msg(e.to_string()))?;
        return Ok(RecordsImportLedgerReport {
            preview,
            apply: None,
            migration: Some(migration),
        });
    }
    let guard = MigrationCommitGuard::begin(
        MigrationKind::Records,
        root,
        &[(store.path(), "records.sqlite")],
    )
    .map_err(|e| RecordsStoreError::Msg(e.to_string()))?;
    let apply = store.import_dir(root)?;
    let migration = guard
        .finalize(
            MigrationKind::Records,
            root,
            apply.inserted as u64,
            apply.skipped as u64,
            apply.errors.len() as u64,
            apply.warnings.clone(),
        )
        .map_err(|e| RecordsStoreError::Msg(e.to_string()))?;
    Ok(RecordsImportLedgerReport {
        preview,
        apply: Some(apply),
        migration: Some(migration),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn seed_category(store: &RecordsStore, name: &str) {
        let conn = store.connect().unwrap();
        let now = now_iso();
        upsert_category_tx(&conn, name, "", &now).unwrap();
    }

    #[test]
    fn schema_init_and_empty_stats() {
        let dir = tempdir().unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        let stats = store.stats().unwrap();
        assert_eq!(stats.categories, 0);
        assert_eq!(stats.records, 0);
    }

    #[test]
    fn insert_search_delete_round_trip() {
        let dir = tempdir().unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        seed_category(&store, "电影");
        let row = store
            .insert_record("电影", "沙丘", Some(8), "史诗")
            .unwrap();
        assert_eq!(row.rating, Some(8));
        let hits = store.search("沙丘", None, 10).unwrap();
        assert_eq!(hits.len(), 1);
        store.delete(row.id).unwrap();
        assert!(store.get(row.id).unwrap().is_none());
    }

    #[test]
    fn duplicate_name_rejected() {
        let dir = tempdir().unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        seed_category(&store, "电影");
        store.insert_record("电影", "A", None, "").unwrap();
        let err = store.insert_record("电影", "A", None, "").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn import_empty_tables_creates_categories() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("电影.md"),
            "# 电影\n\n| 名字 | 评分 | 备注 |\n|---|---:|---|\n",
        )
        .unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        let preview = store.preview_import_dir(&root).unwrap();
        assert_eq!(preview.files_found, 1);
        assert_eq!(preview.records, 0);
        let applied = store.import_dir(&root).unwrap();
        assert_eq!(applied.inserted, 0);
        assert_eq!(store.stats().unwrap().categories, 1);
    }

    #[test]
    fn import_idempotent() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("电影.md"),
            "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| 沙丘 | 8 | |\n",
        )
        .unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        let first = store.import_dir(&root).unwrap();
        assert_eq!(first.inserted, 1);
        let second = store.import_dir(&root).unwrap();
        assert_eq!(second.inserted, 0);
        assert_eq!(second.skipped, 1);
    }

    #[test]
    fn import_preserves_blank_rating_and_note() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("电影.md"),
            "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| 沙丘 | | 待评分 |\n",
        )
        .unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        let report = store.import_dir(&root).unwrap();
        assert_eq!(report.inserted, 1);
        let row = store
            .get_by_category_and_name("电影", "沙丘")
            .unwrap()
            .unwrap();
        assert_eq!(row.rating, None);
        assert_eq!(row.note, "待评分");
    }

    #[test]
    fn deleted_imported_record_is_not_resurrected() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("电影.md"),
            "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| 沙丘 | 8 | |\n",
        )
        .unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        store.import_dir(&root).unwrap();
        let row = store
            .get_by_category_and_name("电影", "沙丘")
            .unwrap()
            .unwrap();
        store.delete(row.id).unwrap();
        let report = store.import_dir(&root).unwrap();
        assert_eq!(report.inserted, 0);
        assert!(store
            .get_by_category_and_name("电影", "沙丘")
            .unwrap()
            .is_none());
    }

    #[test]
    fn inserting_a_row_before_existing_row_keeps_existing_source_identity() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        let source = root.join("电影.md");
        std::fs::write(
            &source,
            "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| A | 8 | |\n",
        )
        .unwrap();
        let store = RecordsStore::with_path(dir.path().join("records.sqlite")).unwrap();
        store.import_dir(&root).unwrap();
        std::fs::write(
            &source,
            "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| B | 7 | |\n| A | 8 | |\n",
        )
        .unwrap();
        let report = store.import_dir(&root).unwrap();
        assert_eq!(report.inserted, 1);
        assert!(store
            .get_by_category_and_name("电影", "A")
            .unwrap()
            .is_some());
        assert!(store
            .get_by_category_and_name("电影", "B")
            .unwrap()
            .is_some());
    }
}
