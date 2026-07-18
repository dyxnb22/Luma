//! Wordbook SQLite store (vocab + SRS). Paths under LumaNext only.

use crate::paths::{ensure_luma_next_dirs, luma_next_support_dir, PathsError};
use chrono::{Duration, Local, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

const EBBINGHAUS_INTERVALS: [Duration; 9] = [
    Duration::minutes(5),
    Duration::minutes(30),
    Duration::hours(12),
    Duration::days(1),
    Duration::days(2),
    Duration::days(4),
    Duration::days(7),
    Duration::days(15),
    Duration::days(30),
];

const MASTERED_NEXT: &str = "9999-12-31T00:00:00Z";

const WORD_COLS: &str = "id, term, phonetic, meaning, example, category, familiarity,
    review_stage, review_count, wrong_count, last_review_at, next_review_at,
    mastered_at, created_at, updated_at";

#[derive(Debug, Error)]
pub enum WordbookStoreError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Error)]
pub enum WordbookReadOnlyError {
    #[error("wordbook not configured")]
    NotConfigured,
    #[error("wordbook sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordRow {
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
pub struct WordContent {
    pub term: String,
    pub phonetic: String,
    pub meaning: String,
    pub example: String,
    pub category: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordImportRow {
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WordbookStats {
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
pub struct ImportContentReport {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WordpetImportReport {
    pub would_insert: usize,
    pub would_update: usize,
    pub skipped: usize,
    pub settings_copied: Vec<String>,
    pub sample_terms: Vec<String>,
    pub committed: bool,
}

pub struct WordbookStore {
    path: PathBuf,
}

/// Read-only view of an existing Wordbook database for lightweight companion processes.
/// Construction never creates the file, parent directories, schema, indexes, or defaults.
pub struct WordbookReadOnlyStore {
    path: PathBuf,
}

impl WordbookReadOnlyStore {
    pub fn with_path(path: PathBuf) -> Result<Self, WordbookReadOnlyError> {
        if !path.is_file() {
            return Err(WordbookReadOnlyError::NotConfigured);
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn stats(&self) -> Result<WordbookStats, WordbookReadOnlyError> {
        let conn = crate::sqlite::open_readonly_connection(&self.path)?;
        Ok(stats_on(&conn)?)
    }
}

impl WordbookStore {
    pub fn luma_next_default() -> Result<Self, WordbookStoreError> {
        ensure_luma_next_dirs()?;
        Self::with_path(luma_next_support_dir()?.join("wordbook.sqlite"))
    }

    pub fn with_path(path: PathBuf) -> Result<Self, WordbookStoreError> {
        let store = Self { path };
        store.init()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> Result<Connection, WordbookStoreError> {
        crate::sqlite::open_connection(&self.path).map_err(Into::into)
    }

    fn init(&self) -> Result<(), WordbookStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                term TEXT NOT NULL UNIQUE,
                phonetic TEXT NOT NULL DEFAULT '',
                meaning TEXT NOT NULL DEFAULT '',
                example TEXT NOT NULL DEFAULT '',
                category TEXT NOT NULL DEFAULT '',
                familiarity TEXT NOT NULL DEFAULT 'unknown',
                review_stage INTEGER NOT NULL DEFAULT 0,
                review_count INTEGER NOT NULL DEFAULT 0,
                wrong_count INTEGER NOT NULL DEFAULT 0,
                last_review_at TEXT NOT NULL DEFAULT '',
                next_review_at TEXT NOT NULL DEFAULT '',
                mastered_at TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_words_next_review_at ON words(next_review_at);
            CREATE INDEX IF NOT EXISTS idx_words_familiarity ON words(familiarity);
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        ensure_fts(&conn)?;
        let now = now_iso();
        conn.execute(
            "UPDATE words SET next_review_at = ?1 WHERE next_review_at = '' OR next_review_at IS NULL",
            params![now],
        )?;
        if conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'daily_goal'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            == 0
        {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('daily_goal', '30')",
                [],
            )?;
        }
        if conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'voice_accent'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            == 0
        {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('voice_accent', 'uk')",
                [],
            )?;
        }
        Ok(())
    }

    pub fn setting(&self, key: &str, default: &str) -> Result<String, WordbookStoreError> {
        let conn = self.connect()?;
        Ok(setting_on(&conn, key, default)?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), WordbookStoreError> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn daily_goal(&self) -> Result<i64, WordbookStoreError> {
        let raw = self.setting("daily_goal", "30")?;
        Ok(raw.parse::<i64>().unwrap_or(30).max(1))
    }

    pub fn set_daily_goal(&self, value: i64) -> Result<(), WordbookStoreError> {
        self.set_setting("daily_goal", &value.max(1).to_string())
    }

    pub fn get(&self, id: i64) -> Result<Option<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                &format!("SELECT {WORD_COLS} FROM words WHERE id = ?1"),
                params![id],
                map_word_row,
            )
            .optional()?;
        Ok(row)
    }

    pub fn get_by_term(&self, term: &str) -> Result<Option<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                &format!("SELECT {WORD_COLS} FROM words WHERE term = ?1"),
                params![term],
                map_word_row,
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_due(&self, limit: usize) -> Result<Vec<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let now = now_iso();
        let mut stmt = conn.prepare(&format!(
            "SELECT {WORD_COLS}
             FROM words
             WHERE mastered_at = '' AND review_count > 0
               AND julianday(next_review_at) <= julianday(?1)
             ORDER BY CASE WHEN wrong_count >= 2 THEN 0 ELSE 1 END,
                      julianday(next_review_at) ASC, wrong_count DESC, review_count ASC
             LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![now, limit as i64], map_word_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_new(&self, limit: usize) -> Result<Vec<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {WORD_COLS}
             FROM words
             WHERE mastered_at = '' AND review_count = 0
             ORDER BY created_at ASC, id ASC
             LIMIT ?1"
        ))?;
        let rows = stmt
            .query_map(params![limit as i64], map_word_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_wrong(&self, limit: usize) -> Result<Vec<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {WORD_COLS}
             FROM words
             WHERE mastered_at = '' AND review_count > 0 AND wrong_count > 0
             ORDER BY wrong_count DESC, julianday(next_review_at) ASC
             LIMIT ?1"
        ))?;
        let rows = stmt
            .query_map(params![limit as i64], map_word_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<WordRow>, WordbookStoreError> {
        let conn = self.connect()?;
        let q = query.trim();
        if q.is_empty() {
            let mut stmt = conn.prepare(&format!(
                "SELECT {WORD_COLS}
                 FROM words
                 ORDER BY julianday(next_review_at) ASC, updated_at DESC
                 LIMIT ?1"
            ))?;
            let rows = stmt
                .query_map(params![limit as i64], map_word_row)?
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(rows);
        }
        // Prefer FTS when available; fall back to LIKE.
        let fts_q = escape_fts_query(q);
        if !fts_q.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT w.id, w.term, w.phonetic, w.meaning, w.example, w.category, w.familiarity,
                        w.review_stage, w.review_count, w.wrong_count, w.last_review_at, w.next_review_at,
                        w.mastered_at, w.created_at, w.updated_at
                 FROM words_fts f
                 JOIN words w ON w.id = f.rowid
                 WHERE words_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            match stmt
                .query_map(params![fts_q, limit as i64], map_word_row)
                .and_then(|iter| iter.collect::<Result<Vec<_>, _>>())
            {
                Ok(rows) if !rows.is_empty() => return Ok(rows),
                Ok(_) | Err(_) => {}
            }
        }
        let like = format!("%{q}%");
        let mut stmt = conn.prepare(&format!(
            "SELECT {WORD_COLS}
             FROM words
             WHERE term LIKE ?1 OR meaning LIKE ?1 OR example LIKE ?1 OR category LIKE ?1
             ORDER BY updated_at DESC
             LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![like, limit as i64], map_word_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn stats(&self) -> Result<WordbookStats, WordbookStoreError> {
        let conn = self.connect()?;
        Ok(stats_on(&conn)?)
    }

    /// Upsert content fields only — never resets SRS progress.
    pub fn upsert_content(&self, content: &WordContent) -> Result<bool, WordbookStoreError> {
        let conn = self.connect()?;
        upsert_content_on(&conn, content)
    }

    /// Batch content upsert in a single transaction (CSV/paste import).
    /// Mirrors the WordPet import commit path so a mid-batch failure rolls back.
    pub fn upsert_contents(
        &self,
        rows: &[WordContent],
    ) -> Result<ImportContentReport, WordbookStoreError> {
        let mut report = ImportContentReport::default();
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        for row in rows {
            if row.term.trim().is_empty() {
                report.skipped += 1;
                continue;
            }
            if upsert_content_on(&tx, row)? {
                report.inserted += 1;
            } else {
                report.updated += 1;
            }
        }
        tx.commit()?;
        Ok(report)
    }

    pub fn delete(&self, id: i64) -> Result<(), WordbookStoreError> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM words WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn review(&self, id: i64, familiarity: &str) -> Result<WordRow, WordbookStoreError> {
        let word = self
            .get(id)?
            .ok_or_else(|| WordbookStoreError::Msg(format!("word {id} not found")))?;
        self.commit_review(word, familiarity)
    }

    /// Apply a review using a previously read snapshot (`updated_at` CAS).
    fn commit_review(
        &self,
        word: WordRow,
        familiarity: &str,
    ) -> Result<WordRow, WordbookStoreError> {
        let id = word.id;
        if !word.mastered_at.is_empty() || word.familiarity == "mastered" {
            return Err(WordbookStoreError::Msg(format!(
                "word {id} is mastered; unmaster before reviewing"
            )));
        }
        let (stage, next_at, wrong_count, fam) =
            schedule_review(familiarity, word.review_stage, word.wrong_count)?;
        let now = now_iso();
        let revision = new_revision();
        let conn = self.connect()?;
        let n = conn.execute(
            "UPDATE words SET familiarity = ?1, review_stage = ?2, review_count = ?3,
                 wrong_count = ?4, last_review_at = ?5, next_review_at = ?6, updated_at = ?7,
                 mastered_at = ''
             WHERE id = ?8 AND updated_at = ?9",
            params![
                fam,
                stage,
                word.review_count + 1,
                wrong_count,
                now,
                next_at,
                revision,
                id,
                word.updated_at
            ],
        )?;
        if n == 0 {
            return Err(WordbookStoreError::Msg(format!(
                "word {id} was modified concurrently; reload and retry"
            )));
        }
        if fam == "unknown" {
            self.bump_daily_wrong()?;
        }
        self.get(id)?
            .ok_or_else(|| WordbookStoreError::Msg(format!("word {id} missing after review")))
    }

    pub fn set_mastered(&self, id: i64, mastered: bool) -> Result<WordRow, WordbookStoreError> {
        let word = self
            .get(id)?
            .ok_or_else(|| WordbookStoreError::Msg(format!("word {id} not found")))?;
        self.commit_set_mastered(word, mastered)
    }

    /// Toggle mastered using a previously read snapshot (`updated_at` CAS).
    fn commit_set_mastered(
        &self,
        word: WordRow,
        mastered: bool,
    ) -> Result<WordRow, WordbookStoreError> {
        let id = word.id;
        let now = now_iso();
        let revision = new_revision();
        let conn = self.connect()?;
        let n = if mastered {
            let full_stage = EBBINGHAUS_INTERVALS.len() as i64;
            let review_count = (word.review_count + 1).max(full_stage);
            conn.execute(
                "UPDATE words SET familiarity = 'mastered', review_stage = ?1, review_count = ?2,
                     last_review_at = ?3, next_review_at = ?4, mastered_at = ?3, updated_at = ?5
                 WHERE id = ?6 AND updated_at = ?7",
                params![
                    full_stage,
                    review_count,
                    now,
                    MASTERED_NEXT,
                    revision,
                    id,
                    word.updated_at
                ],
            )?
        } else {
            conn.execute(
                "UPDATE words SET familiarity = 'unknown', review_stage = 0, review_count = 0,
                     last_review_at = '', next_review_at = ?1, mastered_at = '', updated_at = ?2
                 WHERE id = ?3 AND updated_at = ?4",
                params![now, revision, id, word.updated_at],
            )?
        };
        if n == 0 {
            return Err(WordbookStoreError::Msg(format!(
                "word {id} was modified concurrently; reload and retry"
            )));
        }
        self.get(id)?
            .ok_or_else(|| WordbookStoreError::Msg(format!("word {id} missing after mastered")))
    }

    fn bump_daily_wrong(&self) -> Result<(), WordbookStoreError> {
        self.reset_daily_stats_if_needed()?;
        let n: i64 = self.setting("daily_wrong_count", "0")?.parse().unwrap_or(0);
        self.set_setting("daily_wrong_count", &(n + 1).to_string())
    }

    fn reset_daily_stats_if_needed(&self) -> Result<(), WordbookStoreError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let stored = self.setting("daily_stats_date", "")?;
        if stored == today {
            return Ok(());
        }
        self.set_setting("daily_stats_date", &today)?;
        self.set_setting("daily_new_seen", "0")?;
        self.set_setting("daily_wrong_count", "0")?;
        Ok(())
    }

    pub fn backup(&self) -> Result<PathBuf, WordbookStoreError> {
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
        let dest = backups.join(format!("wordbook-backup-{stamp}.sqlite"));
        let tmp = backups.join(format!("wordbook-backup-{stamp}.sqlite.tmp"));
        let _ = std::fs::remove_file(&tmp);
        let conn = self.connect()?;
        let quoted = sqlite_path_literal(&tmp)?;
        conn.execute_batch(&format!("VACUUM INTO {quoted}"))?;
        std::fs::rename(&tmp, &dest)?;
        Ok(dest)
    }

    /// Dry-run import preview: reads source (and existing dest if present) read-only.
    /// Never creates or writes `wordbook.sqlite`.
    pub fn preview_import_wordpet(
        source: &Path,
    ) -> Result<WordpetImportReport, WordbookStoreError> {
        let dest_path = match luma_next_support_dir() {
            Ok(dir) => {
                let p = dir.join("wordbook.sqlite");
                if p.exists() {
                    Some(p)
                } else {
                    None
                }
            }
            Err(_) => None,
        };
        import_wordpet_inner(source, dest_path.as_deref(), false)
    }

    /// Import from a WordPet/WordBot sqlite file into this store.
    /// Dry-run (`commit=false`) does not write rows/settings; prefer [`Self::preview_import_wordpet`]
    /// from CLI so an empty LumaNext root is not initialized.
    pub fn import_wordpet(
        &self,
        source: &Path,
        commit: bool,
    ) -> Result<WordpetImportReport, WordbookStoreError> {
        import_wordpet_inner(source, Some(self.path()), commit)
    }
}

fn setting_on(conn: &Connection, key: &str, default: &str) -> rusqlite::Result<String> {
    let value = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(value.unwrap_or_else(|| default.to_string()))
}

fn stats_on(conn: &Connection) -> rusqlite::Result<WordbookStats> {
    let now = now_iso();
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM words", [], |r| r.get(0))?;
    let due: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words
         WHERE mastered_at = '' AND review_count > 0
           AND julianday(next_review_at) <= julianday(?1)",
        params![now],
        |r| r.get(0),
    )?;
    let new_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words WHERE mastered_at = '' AND review_count = 0",
        [],
        |r| r.get(0),
    )?;
    let wrong: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words
         WHERE mastered_at = '' AND review_count > 0 AND wrong_count > 0",
        [],
        |r| r.get(0),
    )?;
    let mastered: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words WHERE mastered_at != '' OR familiarity = 'mastered'",
        [],
        |r| r.get(0),
    )?;
    let today_start = today_start_iso();
    let reviewed_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM words WHERE last_review_at >= ?1",
        params![today_start],
        |r| r.get(0),
    )?;
    let goal = setting_on(conn, "daily_goal", "30")?
        .parse::<i64>()
        .unwrap_or(30)
        .max(1);
    Ok(WordbookStats {
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

fn sqlite_path_literal(path: &Path) -> Result<String, WordbookStoreError> {
    let s = path
        .to_str()
        .ok_or_else(|| WordbookStoreError::Msg(format!("non-utf8 path: {}", path.display())))?;
    Ok(format!("'{}'", s.replace('\'', "''")))
}

fn is_wordpet_setting_key(key: &str) -> bool {
    matches!(
        key,
        "daily_goal"
            | "voice_accent"
            | "daily_stats_date"
            | "daily_new_seen"
            | "daily_wrong_count"
            | "paused_until"
    )
}

fn load_wordpet_settings(
    src: &rusqlite::Connection,
) -> Result<Vec<(String, String)>, WordbookStoreError> {
    let mut stmt = src.prepare("SELECT key, value FROM settings")?;
    let settings = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(settings)
}

fn copy_wordpet_settings(
    tx: Option<&rusqlite::Connection>,
    settings: &[(String, String)],
    report: &mut WordpetImportReport,
    commit: bool,
) -> Result<(), WordbookStoreError> {
    for (key, value) in settings {
        if key.starts_with("pet_") || !is_wordpet_setting_key(key) {
            continue;
        }
        report.settings_copied.push(key.clone());
        if commit {
            let tx = tx.expect("commit requires transaction");
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
    }
    Ok(())
}

fn import_wordpet_inner(
    source: &Path,
    dest_path: Option<&Path>,
    commit: bool,
) -> Result<WordpetImportReport, WordbookStoreError> {
    if !source.exists() {
        return Err(WordbookStoreError::Msg(format!(
            "source not found: {}",
            source.display()
        )));
    }
    let src = crate::sqlite::open_readonly_connection(source)
        .map_err(|e| WordbookStoreError::Msg(format!("open source read-only: {e}")))?;
    let mut stmt = src.prepare(
        "SELECT term, phonetic, meaning, example, category, familiarity,
                review_stage, review_count, wrong_count, last_review_at, next_review_at,
                mastered_at, created_at, updated_at
         FROM words ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WordImportRow {
                term: r.get::<_, String>(0).unwrap_or_default(),
                phonetic: r.get::<_, String>(1).unwrap_or_default(),
                meaning: r.get::<_, String>(2).unwrap_or_default(),
                example: r.get::<_, String>(3).unwrap_or_default(),
                category: r.get::<_, String>(4).unwrap_or_default(),
                familiarity: r.get::<_, String>(5).unwrap_or_else(|_| "unknown".into()),
                review_stage: r.get::<_, i64>(6).unwrap_or(0),
                review_count: r.get::<_, i64>(7).unwrap_or(0),
                wrong_count: r.get::<_, i64>(8).unwrap_or(0),
                last_review_at: r.get::<_, String>(9).unwrap_or_default(),
                next_review_at: r.get::<_, String>(10).unwrap_or_default(),
                mastered_at: r.get::<_, String>(11).unwrap_or_default(),
                created_at: r.get::<_, String>(12).unwrap_or_default(),
                updated_at: r.get::<_, String>(13).unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut report = WordpetImportReport {
        committed: commit,
        ..Default::default()
    };

    let dest_ro = if !commit {
        match dest_path {
            Some(p) if p.exists() => Some(
                crate::sqlite::open_readonly_connection(p)
                    .map_err(|e| WordbookStoreError::Msg(format!("open dest read-only: {e}")))?,
            ),
            _ => None,
        }
    } else {
        None
    };

    if commit {
        let dest_path = dest_path.ok_or_else(|| {
            WordbookStoreError::Msg("commit requires a destination wordbook path".into())
        })?;
        // Ensure schema exists (caller should have opened via WordbookStore).
        let dest = crate::sqlite::open_connection(dest_path)?;
        let tx = dest.unchecked_transaction()?;
        for row in &rows {
            if row.term.trim().is_empty() {
                report.skipped += 1;
                continue;
            }
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM words WHERE term = ?1",
                    params![row.term],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            if exists {
                report.would_update += 1;
            } else {
                report.would_insert += 1;
            }
            if report.sample_terms.len() < 5 {
                report.sample_terms.push(row.term.clone());
            }
            upsert_full_row(&tx, row)?;
        }

        let settings = load_wordpet_settings(&src)?;
        copy_wordpet_settings(Some(&tx), &settings, &mut report, true)?;
        tx.commit()?;
        return Ok(report);
    }

    // Dry-run: count only.
    for row in &rows {
        if row.term.trim().is_empty() {
            report.skipped += 1;
            continue;
        }
        let exists = if let Some(dest) = &dest_ro {
            dest.query_row(
                "SELECT 1 FROM words WHERE term = ?1",
                params![row.term],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false)
        } else {
            false
        };
        if exists {
            report.would_update += 1;
        } else {
            report.would_insert += 1;
        }
        if report.sample_terms.len() < 5 {
            report.sample_terms.push(row.term.clone());
        }
    }

    let settings = load_wordpet_settings(&src)?;
    copy_wordpet_settings(None, &settings, &mut report, false)?;
    Ok(report)
}

/// Content-only upsert on an open connection/transaction. Returns `true` if inserted.
fn upsert_content_on(
    conn: &rusqlite::Connection,
    content: &WordContent,
) -> Result<bool, WordbookStoreError> {
    let term = content.term.trim();
    if term.is_empty() {
        return Err(WordbookStoreError::Msg("term is required".into()));
    }
    let now = now_iso();
    let revision = new_revision();
    let existing: Option<i64> = conn
        .query_row("SELECT id FROM words WHERE term = ?1", params![term], |r| {
            r.get(0)
        })
        .optional()?;
    if existing.is_some() {
        conn.execute(
            "UPDATE words SET phonetic = ?1, meaning = ?2, example = ?3, category = ?4, updated_at = ?5
             WHERE term = ?6",
            params![
                content.phonetic,
                content.meaning,
                content.example,
                content.category,
                revision,
                term
            ],
        )?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO words
             (term, phonetic, meaning, example, category, next_review_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
            params![
                term,
                content.phonetic,
                content.meaning,
                content.example,
                content.category,
                now,
                revision
            ],
        )?;
        Ok(true)
    }
}

fn upsert_full_row(
    conn: &rusqlite::Connection,
    row: &WordImportRow,
) -> Result<(), WordbookStoreError> {
    let now = now_iso();
    let next = if row.next_review_at.trim().is_empty() {
        now.clone()
    } else {
        row.next_review_at.clone()
    };
    let created = if row.created_at.trim().is_empty() {
        now.clone()
    } else {
        row.created_at.clone()
    };
    let updated = if row.updated_at.trim().is_empty() {
        now
    } else {
        row.updated_at.clone()
    };
    conn.execute(
        "INSERT INTO words
         (term, phonetic, meaning, example, category, familiarity, review_stage, review_count,
          wrong_count, last_review_at, next_review_at, mastered_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(term) DO UPDATE SET
           phonetic = excluded.phonetic,
           meaning = excluded.meaning,
           example = excluded.example,
           category = excluded.category,
           familiarity = excluded.familiarity,
           review_stage = excluded.review_stage,
           review_count = excluded.review_count,
           wrong_count = excluded.wrong_count,
           last_review_at = excluded.last_review_at,
           next_review_at = excluded.next_review_at,
           mastered_at = excluded.mastered_at,
           updated_at = excluded.updated_at",
        params![
            row.term,
            row.phonetic,
            row.meaning,
            row.example,
            row.category,
            row.familiarity,
            row.review_stage,
            row.review_count,
            row.wrong_count,
            row.last_review_at,
            next,
            row.mastered_at,
            created,
            updated
        ],
    )?;
    Ok(())
}

pub fn schedule_review(
    familiarity: &str,
    current_stage: i64,
    wrong_count: i64,
) -> Result<(i64, String, i64, String), WordbookStoreError> {
    let now = Utc::now();
    match familiarity {
        "known" => {
            let idx = current_stage.clamp(0, (EBBINGHAUS_INTERVALS.len() as i64) - 1) as usize;
            let stage = (current_stage + 1).min(EBBINGHAUS_INTERVALS.len() as i64);
            let next = (now + EBBINGHAUS_INTERVALS[idx])
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            Ok((stage, next, wrong_count, "known".into()))
        }
        "fuzzy" => {
            let idx = current_stage
                .max(1)
                .min((EBBINGHAUS_INTERVALS.len() as i64) - 1) as usize;
            let next = (now + EBBINGHAUS_INTERVALS[idx])
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            Ok((current_stage, next, wrong_count, "fuzzy".into()))
        }
        "unknown" => {
            let new_wrong = wrong_count + 1;
            let delay = if new_wrong <= 1 {
                EBBINGHAUS_INTERVALS[0]
            } else {
                EBBINGHAUS_INTERVALS[1]
            };
            let next = (now + delay).format("%Y-%m-%dT%H:%M:%SZ").to_string();
            Ok((0, next, new_wrong, "unknown".into()))
        }
        other => Err(WordbookStoreError::Msg(format!(
            "invalid familiarity: {other}"
        ))),
    }
}

pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Opaque optimistic-lock token. Keep the timestamp prefix for useful ordering,
/// but add a UUID so two writers in the same second cannot share a CAS version.
fn new_revision() -> String {
    format!("{}~{}", now_iso(), Uuid::new_v4())
}

fn today_start_iso() -> String {
    Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight")
        .and_local_timezone(Local)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

fn map_word_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<WordRow> {
    Ok(WordRow {
        id: r.get(0)?,
        term: r.get(1)?,
        phonetic: r.get(2)?,
        meaning: r.get(3)?,
        example: r.get(4)?,
        category: r.get(5)?,
        familiarity: r.get(6)?,
        review_stage: r.get(7)?,
        review_count: r.get(8)?,
        wrong_count: r.get(9)?,
        last_review_at: r.get(10)?,
        next_review_at: r.get(11)?,
        mastered_at: r.get(12)?,
        created_at: r.get(13)?,
        updated_at: r.get(14)?,
    })
}

fn ensure_fts(conn: &Connection) -> Result<(), WordbookStoreError> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'words_fts'",
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
        CREATE VIRTUAL TABLE words_fts USING fts5(
            term, meaning, example, category,
            content='words', content_rowid='id'
        );
        INSERT INTO words_fts(rowid, term, meaning, example, category)
          SELECT id, term, meaning, example, category FROM words;
        CREATE TRIGGER words_fts_ai AFTER INSERT ON words BEGIN
          INSERT INTO words_fts(rowid, term, meaning, example, category)
          VALUES (new.id, new.term, new.meaning, new.example, new.category);
        END;
        CREATE TRIGGER words_fts_ad AFTER DELETE ON words BEGIN
          INSERT INTO words_fts(words_fts, rowid, term, meaning, example, category)
          VALUES ('delete', old.id, old.term, old.meaning, old.example, old.category);
        END;
        CREATE TRIGGER words_fts_au AFTER UPDATE ON words BEGIN
          INSERT INTO words_fts(words_fts, rowid, term, meaning, example, category)
          VALUES ('delete', old.id, old.term, old.meaning, old.example, old.category);
          INSERT INTO words_fts(rowid, term, meaning, example, category)
          VALUES (new.id, new.term, new.meaning, new.example, new.category);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn readonly_store_does_not_initialize_missing_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("wordbook.sqlite");
        assert!(matches!(
            WordbookReadOnlyStore::with_path(path.clone()),
            Err(WordbookReadOnlyError::NotConfigured)
        ));
        assert!(!path.exists());
        assert!(!path.parent().unwrap().exists());
    }

    #[test]
    fn readonly_store_reads_stats_without_mutating_database() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("wordbook.sqlite");
        let writable = WordbookStore::with_path(path.clone()).unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        let expected = writable.stats().unwrap();
        drop(writable);

        let readonly = WordbookReadOnlyStore::with_path(path.clone()).unwrap();
        assert_eq!(readonly.stats().unwrap(), expected);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), before);
    }

    #[test]
    fn upsert_review_and_stats() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        assert!(store
            .upsert_content(&WordContent {
                term: "latency".into(),
                phonetic: "/x/".into(),
                meaning: "延迟".into(),
                example: "High latency".into(),
                category: "sys".into(),
            })
            .unwrap());
        let word = store.get_by_term("latency").unwrap().unwrap();
        assert_eq!(word.review_count, 0);
        let reviewed = store.review(word.id, "known").unwrap();
        assert_eq!(reviewed.familiarity, "known");
        assert_eq!(reviewed.review_count, 1);
        assert!(reviewed.review_stage >= 1);
        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.goal, 30);
    }

    #[test]
    fn upsert_contents_batch_inserts_and_updates() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        let report = store
            .upsert_contents(&[
                WordContent {
                    term: "alpha".into(),
                    phonetic: "".into(),
                    meaning: "a".into(),
                    example: "".into(),
                    category: "".into(),
                },
                WordContent {
                    term: "".into(),
                    phonetic: "".into(),
                    meaning: "skip".into(),
                    example: "".into(),
                    category: "".into(),
                },
                WordContent {
                    term: "beta".into(),
                    phonetic: "".into(),
                    meaning: "b".into(),
                    example: "".into(),
                    category: "".into(),
                },
            ])
            .unwrap();
        assert_eq!(report.inserted, 2);
        assert_eq!(report.skipped, 1);
        let again = store
            .upsert_contents(&[WordContent {
                term: "alpha".into(),
                phonetic: "".into(),
                meaning: "a2".into(),
                example: "".into(),
                category: "".into(),
            }])
            .unwrap();
        assert_eq!(again.updated, 1);
        assert_eq!(store.get_by_term("alpha").unwrap().unwrap().meaning, "a2");
        assert_eq!(store.stats().unwrap().total, 2);
    }

    #[test]
    fn review_cas_rejects_stale_snapshot() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "cas".into(),
                phonetic: "".into(),
                meaning: "x".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let snapshot = store.get_by_term("cas").unwrap().unwrap();
        // Concurrent writer with a distinct updated_at (second-precision now_iso can collide).
        store
            .connect()
            .unwrap()
            .execute(
                "UPDATE words SET updated_at = '2099-01-01T00:00:00Z' WHERE id = ?1",
                params![snapshot.id],
            )
            .unwrap();
        let err = store
            .commit_review(snapshot, "known")
            .unwrap_err()
            .to_string();
        assert!(err.contains("concurrently"), "{err}");
    }

    #[test]
    fn review_cas_rejects_same_second_stale_snapshot() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "same-second".into(),
                phonetic: "".into(),
                meaning: "x".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let snapshot = store.get_by_term("same-second").unwrap().unwrap();
        let winner = store.review(snapshot.id, "known").unwrap();
        assert_ne!(winner.updated_at, snapshot.updated_at);

        let err = store
            .commit_review(snapshot, "known")
            .unwrap_err()
            .to_string();
        assert!(err.contains("concurrently"), "{err}");
        assert_eq!(store.get(winner.id).unwrap().unwrap().review_count, 1);
    }

    #[test]
    fn set_mastered_cas_rejects_stale_snapshot() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "cas2".into(),
                phonetic: "".into(),
                meaning: "x".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let snapshot = store.get_by_term("cas2").unwrap().unwrap();
        store
            .connect()
            .unwrap()
            .execute(
                "UPDATE words SET updated_at = '2099-01-01T00:00:00Z' WHERE id = ?1",
                params![snapshot.id],
            )
            .unwrap();
        let err = store
            .commit_set_mastered(snapshot, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("concurrently"), "{err}");
    }

    #[test]
    fn content_upsert_preserves_progress() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "foo".into(),
                phonetic: "".into(),
                meaning: "a".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let id = store.get_by_term("foo").unwrap().unwrap().id;
        store.review(id, "known").unwrap();
        store
            .upsert_content(&WordContent {
                term: "foo".into(),
                phonetic: "".into(),
                meaning: "b".into(),
                example: "ex".into(),
                category: "c".into(),
            })
            .unwrap();
        let word = store.get_by_term("foo").unwrap().unwrap();
        assert_eq!(word.meaning, "b");
        assert_eq!(word.review_count, 1);
        assert_eq!(word.familiarity, "known");
    }

    #[test]
    fn mastered_excluded_from_due() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "bar".into(),
                phonetic: "".into(),
                meaning: "x".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let id = store.get_by_term("bar").unwrap().unwrap().id;
        store.review(id, "known").unwrap();
        // Force due.
        {
            let conn = store.connect().unwrap();
            conn.execute(
                "UPDATE words SET next_review_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
                params![id],
            )
            .unwrap();
        }
        assert_eq!(store.list_due(10).unwrap().len(), 1);
        store.set_mastered(id, true).unwrap();
        assert!(store.list_due(10).unwrap().is_empty());
    }

    #[test]
    fn backup_writes_under_lumanext_override() {
        let dir = tempdir().unwrap();
        let _env = crate::paths::LumaNextTestEnvGuard::override_paths(
            dir.path(),
            &dir.path().join("logs"),
        );
        let store = WordbookStore::luma_next_default().unwrap();
        store
            .upsert_content(&WordContent {
                term: "z".into(),
                phonetic: "".into(),
                meaning: "m".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let backup = store.backup().unwrap();
        assert!(backup.exists());
        assert!(backup.to_string_lossy().contains("wordbook-backup-"));
    }

    #[test]
    fn review_rejects_mastered_word() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "bar".into(),
                phonetic: "".into(),
                meaning: "x".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let id = store.get_by_term("bar").unwrap().unwrap().id;
        store.set_mastered(id, true).unwrap();
        let err = store.review(id, "known").unwrap_err().to_string();
        assert!(err.contains("mastered"), "{err}");
    }

    #[test]
    fn wrong_list_excludes_new_and_mastered() {
        let dir = tempdir().unwrap();
        let store = WordbookStore::with_path(dir.path().join("wb.sqlite")).unwrap();
        store
            .upsert_content(&WordContent {
                term: "newish".into(),
                phonetic: "".into(),
                meaning: "n".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        store
            .upsert_content(&WordContent {
                term: "hard".into(),
                phonetic: "".into(),
                meaning: "h".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let hard_id = store.get_by_term("hard").unwrap().unwrap().id;
        store.review(hard_id, "unknown").unwrap();
        store
            .upsert_content(&WordContent {
                term: "done".into(),
                phonetic: "".into(),
                meaning: "d".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        let done_id = store.get_by_term("done").unwrap().unwrap().id;
        store.review(done_id, "unknown").unwrap();
        store.set_mastered(done_id, true).unwrap();
        let wrong = store.list_wrong(50).unwrap();
        assert_eq!(wrong.len(), 1);
        assert_eq!(wrong[0].term, "hard");
        assert_eq!(store.stats().unwrap().wrong, 1);
    }

    #[test]
    fn dry_run_import_is_readonly_on_source_and_dest() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("wordpet.sqlite3");
        {
            let src = WordbookStore::with_path(src_path.clone()).unwrap();
            src.upsert_content(&WordContent {
                term: "throughput".into(),
                phonetic: "".into(),
                meaning: "吞吐量".into(),
                example: "".into(),
                category: "".into(),
            })
            .unwrap();
        }
        {
            let conn = crate::sqlite::open_connection(&src_path).unwrap();
            let _ = conn.pragma_update(None, "journal_mode", "DELETE");
        }
        let wal = PathBuf::from(format!("{}-wal", src_path.display()));
        let shm = PathBuf::from(format!("{}-shm", src_path.display()));
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(&shm);

        let dest = WordbookStore::with_path(dir.path().join("dest.sqlite")).unwrap();
        let dry = dest.import_wordpet(&src_path, false).unwrap();
        assert!(!dry.committed);
        assert_eq!(dry.would_insert, 1);
        assert_eq!(dest.stats().unwrap().total, 0);
        assert!(!wal.exists(), "dry-run must not create source -wal");
        assert!(!shm.exists(), "dry-run must not create source -shm");
    }

    #[test]
    fn import_wordpet_dry_run_and_commit() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("wordpet.sqlite3");
        {
            let src = WordbookStore::with_path(src_path.clone()).unwrap();
            src.upsert_content(&WordContent {
                term: "throughput".into(),
                phonetic: "".into(),
                meaning: "吞吐量".into(),
                example: "Improved throughput".into(),
                category: "sys".into(),
            })
            .unwrap();
            let id = src.get_by_term("throughput").unwrap().unwrap().id;
            src.review(id, "known").unwrap();
            src.set_daily_goal(42).unwrap();
        }
        let dest = WordbookStore::with_path(dir.path().join("dest.sqlite")).unwrap();
        let dry = dest.import_wordpet(&src_path, false).unwrap();
        assert!(!dry.committed);
        assert_eq!(dry.would_insert, 1);
        assert_eq!(dest.stats().unwrap().total, 0);
        let committed = dest.import_wordpet(&src_path, true).unwrap();
        assert!(committed.committed);
        let word = dest.get_by_term("throughput").unwrap().unwrap();
        assert_eq!(word.familiarity, "known");
        assert_eq!(word.review_count, 1);
        assert_eq!(dest.daily_goal().unwrap(), 42);
    }
}
