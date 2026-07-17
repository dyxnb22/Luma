mod import_parse;

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use import_parse::{parse_csv, parse_text};
use luma_application::{
    ActionOutcome, ActionRequest, BoundedUtf8FileReadError, BoundedUtf8FileReaderPort,
    FakeBoundedUtf8FileReader, FakeSpeech, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    SearchMode, SearchSink, SpeechAccent, SpeechPort, WarmupContext, WordContentInput, WordEntry,
    WordbookRepository,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const CSV_IMPORT_MAX_BYTES: usize = 512 * 1024;

pub struct WordbookModule {
    manifest: ModuleManifest,
    store: Arc<dyn WordbookRepository>,
    pasteboard: Arc<dyn PasteboardPort>,
    speech: Arc<dyn SpeechPort>,
    file_reader: Arc<dyn BoundedUtf8FileReaderPort>,
    store_error: RwLock<Option<String>>,
}

impl WordbookModule {
    pub fn with_deps(
        store: Arc<dyn WordbookRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
        speech: Arc<dyn SpeechPort>,
        file_reader: Arc<dyn BoundedUtf8FileReaderPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wordbook"),
                display_name: "Wordbook".into(),
                triggers: vec!["wb".into(), "wordbook".into(), "words".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("W".into()),
                    suggested_query: Some("/wb due".into()),
                    empty_hint: Some(
                        "/wb due · /wb new · /wb wrong · /wb status · /wb add TERM | meaning"
                            .into(),
                    ),
                    supports_browse: false,
                },
            },
            store,
            pasteboard,
            speech,
            file_reader,
            store_error: RwLock::new(None),
        }
    }

    /// Test helper with fake speech.
    pub fn with_store_for_tests(
        store: Arc<dyn WordbookRepository>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self::with_deps(
            store,
            pasteboard,
            Arc::new(FakeSpeech::new()),
            Arc::new(FakeBoundedUtf8FileReader::default()),
        )
    }

    fn word_dto(word: &WordEntry, score: f64) -> SearchItemDto {
        let subtitle = if word.meaning.is_empty() {
            format!(
                "{} · stage {} · next {}",
                word.familiarity, word.review_stage, word.next_review_at
            )
        } else {
            truncate(&word.meaning, 72)
        };
        let mastered = Self::is_mastered(word);
        SearchItemDto {
            id: format!("wb:{}", word.id),
            module_id: "luma.wordbook".into(),
            title: word.term.clone(),
            subtitle: Some(subtitle),
            kind: if mastered {
                "word_mastered".into()
            } else {
                "word".into()
            },
            score,
            primary_action_id: if mastered {
                "unmaster".into()
            } else {
                "known".into()
            },
            primary_action_label: if mastered {
                "Unmaster".into()
            } else {
                "Known".into()
            },
            ..Default::default()
        }
    }

    fn preview_text(word: &WordEntry) -> String {
        let mut lines = vec![word.term.clone()];
        if !word.phonetic.is_empty() {
            lines.push(word.phonetic.clone());
        }
        if !word.meaning.is_empty() {
            lines.push(word.meaning.clone());
        }
        if !word.example.is_empty() {
            lines.push(format!("ex: {}", word.example));
        }
        if !word.category.is_empty() {
            lines.push(format!("category: {}", word.category));
        }
        lines.push(format!(
            "familiarity={} stage={} wrong={} next={}",
            word.familiarity, word.review_stage, word.wrong_count, word.next_review_at
        ));
        lines.join("\n")
    }

    async fn probe_store(&self) -> Result<(), String> {
        match self.store.stats() {
            Ok(_) => {
                *self.store_error.write().await = None;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                *self.store_error.write().await = Some(msg.clone());
                Err(msg)
            }
        }
    }

    fn parse_word_id(id: &str) -> Option<i64> {
        id.strip_prefix("wb:")?.parse().ok()
    }

    fn is_mastered(word: &WordEntry) -> bool {
        !word.mastered_at.is_empty() || word.familiarity == "mastered"
    }

    fn unavailable_outcome(err: impl std::fmt::Display) -> ActionOutcome {
        ActionOutcome::Failed {
            kind: FailureKind::Unavailable {
                reason: err.to_string(),
                retryable: true,
            },
        }
    }

    fn csv_read_failure(error: BoundedUtf8FileReadError) -> ActionOutcome {
        let kind = match error {
            BoundedUtf8FileReadError::Unavailable => FailureKind::Unavailable {
                reason: "CSV file is unavailable".into(),
                retryable: true,
            },
            BoundedUtf8FileReadError::InvalidFile => FailureKind::InvalidInput {
                field: "csv".into(),
                message: "CSV must be a regular non-symlink file".into(),
            },
            BoundedUtf8FileReadError::TooLarge => FailureKind::InvalidInput {
                field: "csv".into(),
                message: format!(
                    "CSV exceeds the {} KiB import limit",
                    CSV_IMPORT_MAX_BYTES / 1024
                ),
            },
            BoundedUtf8FileReadError::InvalidUtf8 => FailureKind::InvalidInput {
                field: "csv".into(),
                message: "CSV must be valid UTF-8".into(),
            },
        };
        ActionOutcome::Failed { kind }
    }

    fn get_word_or_outcome(
        store: &dyn WordbookRepository,
        id: i64,
        entity: &str,
    ) -> Result<WordEntry, ActionOutcome> {
        match store.get(id) {
            Ok(Some(w)) => Ok(w),
            Ok(None) => Err(ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: entity.into(),
                },
            }),
            Err(err) => Err(Self::unavailable_outcome(err)),
        }
    }

    fn term_exists(store: &dyn WordbookRepository, term: &str) -> Result<bool, ActionOutcome> {
        match store.get_by_term(term) {
            Ok(opt) => Ok(opt.is_some()),
            Err(err) => Err(Self::unavailable_outcome(err)),
        }
    }

    async fn send_unavailable(sink: &SearchSink, err: &str) {
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "wb:unavailable".into(),
                    module_id: "luma.wordbook".into(),
                    title: "Wordbook store unavailable".into(),
                    subtitle: Some(crate::ux::friendly_store_error(err)),
                    kind: "unavailable".into(),
                    score: 0.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "Unavailable".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }
}

#[async_trait]
impl LumaModule for WordbookModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.probe_store().await {
            Ok(()) => ModuleState::Ready,
            Err(err) => ModuleState::Failed(err),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let rest = query.rest_raw();
        let rest_norm = query.rest_normalized();

        if let Some(err) = self.store_error.read().await.clone() {
            Self::send_unavailable(&sink, &err).await;
            return;
        }

        // /wb add TERM | meaning [| example [| category]]
        if let Some(payload) = rest
            .strip_prefix("add ")
            .or_else(|| rest.strip_prefix("add\t"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let parts: Vec<&str> = payload.split('|').map(str::trim).collect();
            let term = parts.first().copied().unwrap_or("").trim();
            if !term.is_empty() {
                let meaning = parts.get(1).copied().unwrap_or("").to_string();
                let example = parts.get(2).copied().unwrap_or("").to_string();
                let category = parts.get(3).copied().unwrap_or("").to_string();
                let exists = match Self::term_exists(self.store.as_ref(), term) {
                    Ok(exists) => exists,
                    Err(ActionOutcome::Failed {
                        kind: FailureKind::Unavailable { reason, .. },
                        ..
                    }) => {
                        Self::send_unavailable(&sink, &reason).await;
                        return;
                    }
                    Err(_) => return,
                };
                let body = format!("{meaning}|{example}|{category}");
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("wb:add:{term}"),
                            module_id: "luma.wordbook".into(),
                            title: if exists {
                                format!("Overwrite word {term}")
                            } else {
                                format!("Add word {term}")
                            },
                            subtitle: Some(body),
                            kind: if exists {
                                "update".into()
                            } else {
                                "create".into()
                            },
                            score: 100.0,
                            primary_action_id: "add".into(),
                            primary_action_label: if exists {
                                "Overwrite".into()
                            } else {
                                "Add".into()
                            },
                            primary_action_risk: if exists {
                                ActionRisk::Confirm
                            } else {
                                ActionRisk::Safe
                            },
                            primary_action_confirmation: exists,
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        // /wb goal N
        if let Some(n) = rest_norm
            .strip_prefix("goal ")
            .map(str::trim)
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|&n| n >= 1)
        {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("wb:goal:{n}"),
                        module_id: "luma.wordbook".into(),
                        title: format!("Set daily goal to {n}"),
                        subtitle: Some("Enter to save".into()),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "set_goal".into(),
                        primary_action_label: "Set goal".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        // /wb import PATH
        if let Some(path) = rest
            .strip_prefix("import ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("wb:import:{path}"),
                        module_id: "luma.wordbook".into(),
                        title: format!("Import CSV {path}"),
                        subtitle: Some("Enter to import (content only; SRS preserved)".into()),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "import_csv".into(),
                        primary_action_label: "Import".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if matches!(rest_norm.as_str(), "paste" | "paste-import") {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "wb:paste".into(),
                        module_id: "luma.wordbook".into(),
                        title: "Import from clipboard".into(),
                        subtitle: Some("Markdown table or `word - meaning - example`".into()),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "import_paste".into(),
                        primary_action_label: "Import".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_norm == "backup" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "wb:backup".into(),
                        module_id: "luma.wordbook".into(),
                        title: "Backup wordbook".into(),
                        subtitle: Some("Copy to LumaNext/backups/".into()),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "backup".into(),
                        primary_action_label: "Backup".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_norm.starts_with("review") {
            let queue = if rest_norm == "review new" {
                "new"
            } else if rest_norm == "review wrong" {
                "wrong"
            } else {
                "due"
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("wb:review:{queue}"),
                        module_id: "luma.wordbook".into(),
                        title: format!("Start review ({queue})"),
                        subtitle: Some(
                            "Enter to start · reveal then 1/2/3/m grade · s skip · Esc exit".into(),
                        ),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "start_review".into(),
                        primary_action_label: "Start review".into(),
                        action_payload: Some(serde_json::json!({ "queue": queue })),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_norm == "status" {
            match self.store.stats() {
                Ok(s) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "wb:status".into(),
                                module_id: "luma.wordbook".into(),
                                title: format!(
                                    "Today {}/{} · due {} · new {} · wrong {}",
                                    s.reviewed_today, s.goal, s.due, s.new_count, s.wrong
                                ),
                                subtitle: Some(format!(
                                    "total {} · mastered {} · remaining goal {}",
                                    s.total, s.mastered, s.remaining_goal
                                )),
                                kind: "status".into(),
                                score: 100.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "OK".into(),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
                Err(err) => {
                    *self.store_error.write().await = Some(err.to_string());
                    Self::send_unavailable(&sink, &err.to_string()).await;
                }
            }
            return;
        }

        let mode = if rest_norm.is_empty() || rest_norm == "due" {
            "due"
        } else if rest_norm == "new" {
            "new"
        } else if rest_norm == "wrong" {
            "wrong"
        } else {
            "search"
        };

        let result = match mode {
            "due" => self.store.list_due(query.limit),
            "new" => self.store.list_new(query.limit),
            "wrong" => self.store.list_wrong(query.limit),
            _ => self.store.search(rest, query.limit),
        };

        let words = match result {
            Ok(w) => w,
            Err(err) => {
                *self.store_error.write().await = Some(err.to_string());
                Self::send_unavailable(&sink, &err.to_string()).await;
                return;
            }
        };

        if cancel.is_cancelled() {
            return;
        }

        let mut upserts: Vec<SearchItemDto> = words
            .iter()
            .enumerate()
            .map(|(i, w)| Self::word_dto(w, 90.0 - i as f64 * 0.1))
            .collect();

        if upserts.is_empty() {
            let (title, subtitle) = match mode {
                "due" => (
                    "No words due".into(),
                    "Try `/wb new` or `/wb status`".into(),
                ),
                "new" => (
                    "No new words".into(),
                    "Add with `/wb add TERM | meaning`".into(),
                ),
                "wrong" => ("No wrong words".into(), "Nice work".into()),
                _ => (
                    format!("No words matching \"{rest}\""),
                    "Try `/wb due` or `/wb add TERM | meaning`".into(),
                ),
            };
            upserts.push(SearchItemDto {
                id: "wb:empty".into(),
                module_id: "luma.wordbook".into(),
                title,
                subtitle: Some(subtitle),
                kind: "status".into(),
                score: 5.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }

        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind == "word" || result.kind == "word_mastered" {
            let id = Self::parse_word_id(result.id.as_str())?;
            return match self.store.get(id) {
                Ok(Some(w)) => Some(Self::preview_text(&w)),
                Ok(None) => result.subtitle.clone(),
                Err(_) => result.subtitle.clone(),
            };
        }
        result
            .subtitle
            .clone()
            .or_else(|| Some(result.title.clone()))
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.primary_action.id.as_str() {
            "add" => vec![ActionDescriptor {
                id: ActionId::new("add"),
                label: result.primary_action.label.clone(),
                risk: if result.primary_action.confirmation {
                    ActionRisk::Confirm
                } else {
                    ActionRisk::Safe
                },
                confirmation: result.primary_action.confirmation,
            }],
            "set_goal" => vec![ActionDescriptor {
                id: ActionId::new("set_goal"),
                label: "Set goal".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "import_csv" => vec![ActionDescriptor {
                id: ActionId::new("import_csv"),
                label: "Import".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "import_paste" => vec![ActionDescriptor {
                id: ActionId::new("import_paste"),
                label: "Import".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "backup" => vec![ActionDescriptor {
                id: ActionId::new("backup"),
                label: "Backup".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "start_review" => vec![ActionDescriptor {
                id: ActionId::new("start_review"),
                label: "Start review".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "noop" => vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            _ if result.kind == "word_mastered" => vec![
                ActionDescriptor {
                    id: ActionId::new("unmaster"),
                    label: "Unmaster".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
                ActionDescriptor {
                    id: ActionId::new("speak"),
                    label: "Speak".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("speak_example"),
                    label: "Speak example".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("copy_term"),
                    label: "Copy term".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("delete"),
                    label: "Delete".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                },
            ],
            _ if result.kind == "word" => vec![
                ActionDescriptor {
                    id: ActionId::new("known"),
                    label: "Known".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("fuzzy"),
                    label: "Fuzzy".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("unknown"),
                    label: "Unknown".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("mastered"),
                    label: "Mastered".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
                ActionDescriptor {
                    id: ActionId::new("speak"),
                    label: "Speak".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("speak_example"),
                    label: "Speak example".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("copy_term"),
                    label: "Copy term".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("delete"),
                    label: "Delete".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                },
            ],
            _ => vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
        }
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success { message: None },
            "start_review" => ActionOutcome::Success {
                message: Some("review session is TUI-only — use Enter on the review row".into()),
            },
            "add" => {
                let Some(term) = action.result.id.as_str().strip_prefix("wb:add:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:add:<term>".into(),
                        },
                    };
                };
                let body = action.result.subtitle.clone().unwrap_or_default();
                let mut parts = body.split('|');
                let meaning = parts.next().unwrap_or("").to_string();
                let example = parts.next().unwrap_or("").to_string();
                let category = parts.next().unwrap_or("").to_string();
                let exists = match Self::term_exists(self.store.as_ref(), term) {
                    Ok(exists) => exists,
                    Err(outcome) => return outcome,
                };
                if exists && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required to overwrite word".into(),
                        },
                    };
                }
                match self.store.upsert_content(&WordContentInput {
                    term: term.into(),
                    phonetic: String::new(),
                    meaning,
                    example,
                    category,
                }) {
                    Ok(inserted) => ActionOutcome::Success {
                        message: Some(if inserted {
                            format!("added {term}")
                        } else {
                            format!("updated {term}")
                        }),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "set_goal" => {
                let Some(n) = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("wb:goal:")
                    .and_then(|s| s.parse::<i64>().ok())
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:goal:<n>".into(),
                        },
                    };
                };
                match self.store.set_daily_goal(n) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("daily goal set to {n}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "import_csv" => {
                let Some(path) = action.result.id.as_str().strip_prefix("wb:import:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:import:<path>".into(),
                        },
                    };
                };
                let path = PathBuf::from(path);
                let text = match await_unless_cancelled(
                    &cancel,
                    self.file_reader.read_utf8(&path, CSV_IMPORT_MAX_BYTES),
                )
                .await
                {
                    None => return ActionOutcome::Cancelled,
                    Some(Ok(text)) => text,
                    Some(Err(error)) => return Self::csv_read_failure(error),
                };
                let parsed = parse_csv(&text);
                if parsed.rows.is_empty() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "csv".into(),
                            message: "no words parsed from CSV (need term/word header)".into(),
                        },
                    };
                }
                match self.store.upsert_contents(&parsed.rows) {
                    Ok(report) => ActionOutcome::Success {
                        message: Some(format!(
                            "imported +{} ~{} skip {}",
                            report.inserted,
                            report.updated,
                            parsed.skipped + report.skipped
                        )),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "import_paste" => {
                let text = match await_unless_cancelled(&cancel, self.pasteboard.read_text()).await
                {
                    None => return ActionOutcome::Cancelled,
                    Some(Ok(Some(t))) => t,
                    Some(Ok(None)) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::InvalidInput {
                                field: "clipboard".into(),
                                message: "clipboard is empty".into(),
                            },
                        };
                    }
                    Some(Err(err)) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: err.to_string(),
                                retryable: true,
                            },
                        };
                    }
                };
                let parsed = parse_text(&text);
                if parsed.rows.is_empty() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "clipboard".into(),
                            message: "no words parsed from clipboard".into(),
                        },
                    };
                }
                match self.store.upsert_contents(&parsed.rows) {
                    Ok(report) => ActionOutcome::Success {
                        message: Some(format!(
                            "pasted +{} ~{} skip {}",
                            report.inserted,
                            report.updated,
                            parsed.skipped + report.skipped
                        )),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "backup" => match self.store.backup() {
                Ok(path) => ActionOutcome::Success {
                    message: Some(format!("backed up to {}", path.display())),
                },
                Err(err) => ActionOutcome::Failed {
                    kind: FailureKind::Io {
                        context: err.to_string(),
                    },
                },
            },
            "known" | "fuzzy" | "unknown" => {
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                let fam = action.action.id.as_str();
                match self.store.review(id, fam) {
                    Ok(word) => ActionOutcome::Success {
                        message: Some(format!(
                            "{} → {} (stage {})",
                            word.term, word.familiarity, word.review_stage
                        )),
                    },
                    Err(err) => {
                        let msg = err.to_string();
                        ActionOutcome::Failed {
                            kind: if msg.contains("mastered") {
                                FailureKind::InvalidInput {
                                    field: "word".into(),
                                    message: msg,
                                }
                            } else if msg.contains("not found") {
                                FailureKind::NotFound {
                                    entity: action.result.id.as_str().into(),
                                }
                            } else {
                                FailureKind::Io { context: msg }
                            },
                        }
                    }
                }
            }
            "mastered" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                match self.store.set_mastered(id, true) {
                    Ok(word) => ActionOutcome::Success {
                        message: Some(format!("{} mastered", word.term)),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "unmaster" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                match self.store.set_mastered(id, false) {
                    Ok(word) => ActionOutcome::Success {
                        message: Some(format!("{} unmarked as mastered", word.term)),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            "speak" | "speak_example" => {
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                let word = match Self::get_word_or_outcome(
                    self.store.as_ref(),
                    id,
                    action.result.id.as_str(),
                ) {
                    Ok(w) => w,
                    Err(outcome) => return outcome,
                };
                let text = if action.action.id.as_str() == "speak_example" {
                    if word.example.is_empty() {
                        return ActionOutcome::Failed {
                            kind: FailureKind::InvalidInput {
                                field: "example".into(),
                                message: "no example to speak".into(),
                            },
                        };
                    }
                    word.example
                } else {
                    word.term
                };
                match await_unless_cancelled(&cancel, self.speech.speak(&text, SpeechAccent::Uk))
                    .await
                {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("spoke".into()),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "copy_term" => {
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                let word = match Self::get_word_or_outcome(
                    self.store.as_ref(),
                    id,
                    action.result.id.as_str(),
                ) {
                    Ok(w) => w,
                    Err(outcome) => return outcome,
                };
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(&word.term)).await
                {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("copied".into()),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "delete" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(id) = Self::parse_word_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected wb:<id>".into(),
                        },
                    };
                };
                let term = match self.store.get(id) {
                    Ok(Some(w)) => w.term,
                    Ok(None) => id.to_string(),
                    Err(err) => {
                        return Self::unavailable_outcome(err);
                    }
                };
                match self.store.delete(id) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted {term}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: other.into(),
                },
            },
        }
    }

    async fn teardown(&self) {
        *self.store_error.write().await = None;
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeBoundedUtf8FileReader, FakePasteboard, FakeSpeech, MemoryWordbookRepository,
        WordbookRepository,
    };
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ResultId, SearchItem};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn import_request(path: &str) -> ActionRequest {
        let action = ActionDescriptor {
            id: ActionId::new("import_csv"),
            label: "Import".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        };
        ActionRequest {
            result: SearchItem {
                id: ResultId::new(format!("wb:import:{path}")),
                module_id: ModuleId::new("luma.wordbook"),
                title: "Import CSV".into(),
                subtitle: None,
                kind: "command".into(),
                score: 1.0,
                primary_action: action.clone(),
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            },
            action,
            confirmation: false,
        }
    }

    fn module_with_reader(
        reader: Arc<FakeBoundedUtf8FileReader>,
    ) -> (WordbookModule, Arc<MemoryWordbookRepository>) {
        let store = Arc::new(MemoryWordbookRepository::new());
        let module = WordbookModule::with_deps(
            store.clone(),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeSpeech::new()),
            reader,
        );
        (module, store)
    }

    #[tokio::test]
    async fn csv_import_read_failure_is_sanitized_and_does_not_write() {
        let reader = Arc::new(FakeBoundedUtf8FileReader::with_error(
            BoundedUtf8FileReadError::Unavailable,
        ));
        let (module, store) = module_with_reader(reader.clone());
        let path = "/restricted/hidden-words.csv";

        let outcome = module
            .perform(import_request(path), CancellationToken::new())
            .await;

        match outcome {
            ActionOutcome::Failed {
                kind: FailureKind::Unavailable { reason, retryable },
            } => {
                assert_eq!(reason, "CSV file is unavailable");
                assert!(retryable);
                assert!(!reason.contains(path));
            }
            other => panic!("expected sanitized unavailable outcome, got {other:?}"),
        }
        assert!(store.get_by_term("latency").unwrap().is_none());
        assert_eq!(reader.calls.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn csv_import_rejects_oversized_input_without_writing() {
        let reader = Arc::new(FakeBoundedUtf8FileReader::with_error(
            BoundedUtf8FileReadError::TooLarge,
        ));
        let (module, store) = module_with_reader(reader);

        let outcome = module
            .perform(import_request("large.csv"), CancellationToken::new())
            .await;

        match outcome {
            ActionOutcome::Failed {
                kind: FailureKind::InvalidInput { field, message },
            } => {
                assert_eq!(field, "csv");
                assert_eq!(message, "CSV exceeds the 512 KiB import limit");
            }
            other => panic!("expected oversized CSV failure, got {other:?}"),
        }
        assert!(store.get_by_term("latency").unwrap().is_none());
    }

    #[tokio::test]
    async fn csv_import_uses_fake_reader_and_imports_content() {
        let reader = Arc::new(FakeBoundedUtf8FileReader::with_text(
            "term,meaning\nlatency,\u{5ef6}\u{8fdf}\n",
        ));
        let (module, store) = module_with_reader(reader.clone());

        let outcome = module
            .perform(import_request("words.csv"), CancellationToken::new())
            .await;

        assert_eq!(
            outcome,
            ActionOutcome::Success {
                message: Some("imported +1 ~0 skip 0".into()),
            }
        );
        let word = store
            .get_by_term("latency")
            .unwrap()
            .expect("imported word");
        assert_eq!(word.meaning, "\u{5ef6}\u{8fdf}");
        assert_eq!(
            reader.calls.lock().expect("lock").as_slice(),
            &[(PathBuf::from("words.csv"), CSV_IMPORT_MAX_BYTES)]
        );
    }
}
