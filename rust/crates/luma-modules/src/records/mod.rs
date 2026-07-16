use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, RecordsRepository,
    SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct RecordsModule {
    manifest: ModuleManifest,
    store: Arc<dyn RecordsRepository>,
    import_root: Arc<RwLock<Option<PathBuf>>>,
    store_error: RwLock<Option<String>>,
}

impl RecordsModule {
    pub fn with_deps(store: Arc<dyn RecordsRepository>, import_root: Option<PathBuf>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.records"),
                display_name: "Records".into(),
                triggers: vec!["rec".into(), "record".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("R".into()),
                    suggested_query: Some("rec ".into()),
                    empty_hint: Some(
                        "rec <query> · rec 电影 browse · rec add 电影 NAME | rating | note".into(),
                    ),
                    supports_browse: true,
                },
            },
            store,
            import_root: Arc::new(RwLock::new(import_root)),
            store_error: RwLock::new(None),
        }
    }

    pub fn with_store_for_tests(store: Arc<dyn RecordsRepository>) -> Self {
        Self::with_deps(store, None)
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

    fn parse_record_id(id: &str) -> Option<i64> {
        id.strip_prefix("rec:")?.parse().ok()
    }

    fn record_dto(record: &luma_application::RecordEntry, score: f64) -> SearchItemDto {
        let rating = record
            .rating
            .map(|r| format!("★{r}"))
            .unwrap_or_else(|| "—".into());
        let note_preview = truncate_note(&record.note, 40);
        let subtitle = if note_preview.is_empty() {
            format!("{} · {}", record.category_name, rating)
        } else {
            format!("{} · {} · {}", record.category_name, rating, note_preview)
        };
        SearchItemDto {
            id: format!("rec:{}", record.id),
            module_id: "luma.records".into(),
            title: record.name.clone(),
            subtitle: Some(subtitle),
            kind: "record".into(),
            score,
            primary_action_id: "open".into(),
            primary_action_label: "View".into(),
            ..Default::default()
        }
    }

    fn preview_text(record: &luma_application::RecordEntry) -> String {
        let mut lines = vec![
            record.name.clone(),
            format!("category: {}", record.category_name),
            format!(
                "rating: {}",
                record
                    .rating
                    .map(|r| format!("{r}/10"))
                    .unwrap_or_else(|| "—".into())
            ),
        ];
        if !record.note.is_empty() {
            lines.push("note:".into());
            lines.push(record.note.clone());
        }
        lines.push(format!("updated: {}", record.updated_at));
        if !record.source_file.is_empty() {
            lines.push(format!(
                "source: {} ({})",
                record.source_file, record.source_key
            ));
        }
        lines.join("\n")
    }

    async fn send_unavailable(sink: &SearchSink, err: &str) {
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "rec:unavailable".into(),
                    module_id: "luma.records".into(),
                    title: "Records store unavailable".into(),
                    subtitle: Some(err.into()),
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

    fn unavailable_outcome(err: impl std::fmt::Display) -> ActionOutcome {
        ActionOutcome::Failed {
            kind: FailureKind::Unavailable {
                reason: err.to_string(),
                retryable: true,
            },
        }
    }

    fn get_record_or_outcome(
        store: &dyn RecordsRepository,
        id: i64,
    ) -> Result<luma_application::RecordEntry, ActionOutcome> {
        match store.get(id) {
            Ok(Some(r)) => Ok(r),
            Ok(None) => Err(ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("record:{id}"),
                },
            }),
            Err(err) => Err(Self::unavailable_outcome(err)),
        }
    }

    fn parse_add_payload(payload: &str) -> Option<(String, String, Option<i64>, String)> {
        let parts: Vec<&str> = payload.splitn(3, '|').map(str::trim).collect();
        let head = parts.first()?.trim();
        if head.is_empty() {
            return None;
        }
        let mut tokens = head.splitn(2, char::is_whitespace);
        let category = tokens.next()?.trim().to_string();
        let name = tokens.next()?.trim().to_string();
        if category.is_empty() || name.is_empty() {
            return None;
        }
        let rating = match parts.get(1).copied().unwrap_or("") {
            "" => None,
            raw => {
                let rating = raw.parse::<i64>().ok()?;
                if !(1..=10).contains(&rating) {
                    return None;
                }
                Some(rating)
            }
        };
        let note = parts.get(2).copied().unwrap_or("").to_string();
        Some((category, name, rating, note))
    }

    fn parse_rate_command(rest: &str) -> Option<(i64, Option<i64>)> {
        let mut tokens = rest.split_whitespace();
        let id = tokens.next()?.parse().ok()?;
        let score = tokens.next()?;
        if score.eq_ignore_ascii_case("clear") {
            return Some((id, None));
        }
        let rating = score.parse().ok()?;
        if (1..=10).contains(&rating) {
            Some((id, Some(rating)))
        } else {
            None
        }
    }
}

fn truncate_note(s: &str, max: usize) -> String {
    let t = s.trim().replace('\n', " ");
    if t.chars().count() <= max {
        t
    } else {
        format!("{}…", t.chars().take(max).collect::<String>())
    }
}

fn category_matches_name(categories: &[luma_application::RecordCategory], token: &str) -> bool {
    categories.iter().any(|c| c.name == token)
}

#[async_trait]
impl LumaModule for RecordsModule {
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
        if let Some(err) = self.store_error.read().await.clone() {
            Self::send_unavailable(&sink, &err).await;
            return;
        }

        let rest = query.rest_raw();
        let rest_norm = query.rest_normalized();

        if rest_norm == "status" {
            match self.store.stats() {
                Ok(s) => {
                    let root = self.import_root.read().await;
                    let root_hint = root
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "luma record import --root PATH".into());
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "rec:status".into(),
                                module_id: "luma.records".into(),
                                title: format!(
                                    "{} categories · {} records",
                                    s.categories, s.records
                                ),
                                subtitle: Some(root_hint),
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
                Err(err) => Self::send_unavailable(&sink, &err.to_string()).await,
            }
            return;
        }

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
                        id: format!("rec:import:{path}"),
                        module_id: "luma.records".into(),
                        title: format!("Import records from {path}"),
                        subtitle: Some("Enter to import · read-only source".into()),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "import".into(),
                        primary_action_label: "Import".into(),
                        primary_action_risk: ActionRisk::Confirm,
                        primary_action_confirmation: true,
                        action_payload: Some(serde_json::json!({ "path": path })),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if let Some(payload) = rest
            .strip_prefix("add ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if let Some((category, name, rating, note)) = Self::parse_add_payload(payload) {
                let exists = self
                    .store
                    .get_by_category_and_name(&category, &name)
                    .ok()
                    .flatten()
                    .is_some();
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("rec:add:{category}:{name}"),
                            module_id: "luma.records".into(),
                            title: if exists {
                                format!("Overwrite {name} in {category}")
                            } else {
                                format!("Add {name} to {category}")
                            },
                            subtitle: Some(format!("{rating:?} · {}", truncate_note(&note, 60))),
                            kind: if exists { "update" } else { "create" }.into(),
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
                            action_payload: Some(serde_json::json!({
                                "category": category,
                                "name": name,
                                "rating": rating,
                                "note": note,
                            })),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
            } else {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "rec:add-usage".into(),
                            module_id: "luma.records".into(),
                            title: "Add a record".into(),
                            subtitle: Some("Usage: rec add 电影 NAME | rating | note".into()),
                            kind: "status".into(),
                            score: 50.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
            }
            return;
        }

        if let Some(rate_rest) = rest_norm.strip_prefix("rate ") {
            if let Some((id, rating)) = Self::parse_rate_command(rate_rest) {
                let label = rating
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "clear".into());
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("rec:rate:{id}:{label}"),
                            module_id: "luma.records".into(),
                            title: format!("Set rating for record {id}"),
                            subtitle: Some(format!("score: {label}")),
                            kind: "command".into(),
                            score: 100.0,
                            primary_action_id: "rate".into(),
                            primary_action_label: "Rate".into(),
                            action_payload: Some(serde_json::json!({ "id": id, "rating": rating })),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        if rest_norm.starts_with("note ") {
            let Some((_, rest)) = rest.split_once(char::is_whitespace) else {
                return;
            };
            let Some((id, text)) = rest.split_once(char::is_whitespace) else {
                return;
            };
            let Ok(id) = id.parse::<i64>() else {
                return;
            };
            let text = text.trim().to_string();
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("rec:note:{id}"),
                        module_id: "luma.records".into(),
                        title: format!("Update note for record {id}"),
                        subtitle: Some(truncate_note(&text, 80)),
                        kind: "command".into(),
                        score: 100.0,
                        primary_action_id: "note".into(),
                        primary_action_label: "Save note".into(),
                        action_payload: Some(serde_json::json!({ "id": id, "text": text })),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let categories = match self.store.list_categories() {
            Ok(c) => c,
            Err(err) => {
                Self::send_unavailable(&sink, &err.to_string()).await;
                return;
            }
        };

        let browse_path = rest_norm
            .strip_prefix("browse ")
            .or_else(|| rest_norm.strip_prefix("ls "))
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if rest_norm == "browse" || rest_norm == "ls" || browse_path.is_some() {
            let mut upserts = Vec::new();
            if let Some(cat) = browse_path {
                match self.store.list_by_category(cat, query.limit) {
                    Ok(rows) => {
                        for (i, row) in rows.into_iter().enumerate() {
                            if cancel.is_cancelled() {
                                return;
                            }
                            upserts.push(Self::record_dto(&row, 90.0 - i as f64 * 0.1));
                        }
                    }
                    Err(err) => {
                        Self::send_unavailable(&sink, &err.to_string()).await;
                        return;
                    }
                }
                if upserts.is_empty() {
                    upserts.push(SearchItemDto {
                        id: "rec:browse-empty".into(),
                        module_id: "luma.records".into(),
                        title: format!("No records in {cat}"),
                        subtitle: Some("rec add {cat} NAME".into()),
                        kind: "status".into(),
                        score: 50.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ..Default::default()
                    });
                }
            } else {
                for (i, cat) in categories.iter().enumerate() {
                    if cancel.is_cancelled() {
                        return;
                    }
                    upserts.push(SearchItemDto {
                        id: format!("rec:cat:{}", cat.name),
                        module_id: "luma.records".into(),
                        title: format!("{}/", cat.name),
                        subtitle: Some("Enter to browse category".into()),
                        kind: "category".into(),
                        score: 85.0 - i as f64 * 0.1,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        action_payload: Some(serde_json::json!({ "category": cat.name })),
                        ..Default::default()
                    });
                }
                if upserts.is_empty() {
                    upserts.push(SearchItemDto {
                        id: "rec:not-configured".into(),
                        module_id: "luma.records".into(),
                        title: "No record categories yet".into(),
                        subtitle: Some(
                            "luma record import --root ~/Documents/Notes/Records --apply".into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "seed_config".into(),
                        primary_action_label: "Show command".into(),
                        ..Default::default()
                    });
                }
            }
            upserts.truncate(query.limit);
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_norm.is_empty() {
            let mut upserts = Vec::new();
            if categories.is_empty() {
                upserts.push(SearchItemDto {
                    id: "rec:not-configured".into(),
                    module_id: "luma.records".into(),
                    title: "Import your Records markdown tables".into(),
                    subtitle: Some(
                        "luma record import --root ~/Documents/Notes/Records --apply".into(),
                    ),
                    kind: "not_configured".into(),
                    score: 0.0,
                    primary_action_id: "seed_config".into(),
                    primary_action_label: "Show command".into(),
                    ..Default::default()
                });
            } else {
                for (i, cat) in categories.iter().enumerate() {
                    upserts.push(SearchItemDto {
                        id: format!("rec:cat:{}", cat.name),
                        module_id: "luma.records".into(),
                        title: cat.name.clone(),
                        subtitle: Some("rec browse · rec <category> <query>".into()),
                        kind: "category".into(),
                        score: 80.0 - i as f64,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        action_payload: Some(serde_json::json!({ "category": cat.name })),
                        ..Default::default()
                    });
                }
            }
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let tokens: Vec<&str> = rest_norm.split_whitespace().collect();
        let (category_filter, search_q) =
            if tokens.len() >= 2 && category_matches_name(&categories, tokens[0]) {
                (Some(tokens[0]), tokens[1..].join(" "))
            } else if tokens.len() == 1 && category_matches_name(&categories, tokens[0]) {
                match self.store.list_by_category(tokens[0], query.limit) {
                    Ok(rows) => {
                        let mut upserts: Vec<_> = rows
                            .iter()
                            .enumerate()
                            .map(|(i, r)| Self::record_dto(r, 88.0 - i as f64 * 0.1))
                            .collect();
                        if upserts.is_empty() {
                            upserts.push(SearchItemDto {
                                id: "rec:browse-empty".into(),
                                module_id: "luma.records".into(),
                                title: format!("No records in {}", tokens[0]),
                                subtitle: Some(format!("rec add {} NAME", tokens[0])),
                                kind: "status".into(),
                                score: 50.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "OK".into(),
                                ..Default::default()
                            });
                        }
                        upserts.truncate(query.limit);
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts,
                                removed_ids: vec![],
                            })
                            .await;
                    }
                    Err(err) => Self::send_unavailable(&sink, &err.to_string()).await,
                }
                return;
            } else {
                (None, rest_norm.clone())
            };

        if search_q.is_empty() {
            return;
        }

        match self.store.search(&search_q, category_filter, query.limit) {
            Ok(rows) => {
                let mut upserts: Vec<_> = rows
                    .iter()
                    .enumerate()
                    .map(|(i, r)| Self::record_dto(r, 95.0 - i as f64 * 0.5))
                    .collect();
                if upserts.is_empty() {
                    upserts.push(SearchItemDto {
                        id: "rec:no-matches".into(),
                        module_id: "luma.records".into(),
                        title: format!("No records matching \"{search_q}\""),
                        subtitle: Some("rec add CATEGORY NAME · rec browse".into()),
                        kind: "status".into(),
                        score: 50.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ..Default::default()
                    });
                }
                upserts.truncate(query.limit);
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts,
                        removed_ids: vec![],
                    })
                    .await;
            }
            Err(err) => Self::send_unavailable(&sink, &err.to_string()).await,
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind == "not_configured" {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "category" || result.primary_action.id.as_str() == "browse" {
            return vec![ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.primary_action.id.as_str() == "add" {
            return vec![ActionDescriptor {
                id: ActionId::new("add"),
                label: result.primary_action.label.clone(),
                risk: result.primary_action.risk.clone(),
                confirmation: result.primary_action.confirmation,
            }];
        }
        if result.primary_action.id.as_str() == "import" {
            return vec![ActionDescriptor {
                id: ActionId::new("import"),
                label: "Import".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }];
        }
        if result.primary_action.id.as_str() == "note" {
            return vec![ActionDescriptor {
                id: ActionId::new("note"),
                label: result.primary_action.label.clone(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.primary_action.id.as_str() == "rate" {
            return vec![ActionDescriptor {
                id: ActionId::new("rate"),
                label: "Rate".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "status"
            || result.kind == "unavailable"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "record" {
            return vec![
                ActionDescriptor {
                    id: ActionId::new("open"),
                    label: "View".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("rate"),
                    label: "Rate".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("note"),
                    label: "Edit note".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("remove"),
                    label: "Remove".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
            ];
        }
        vec![]
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind == "category" {
            return result.subtitle.clone();
        }
        let id = Self::parse_record_id(result.id.as_str())?;
        match self.store.get(id) {
            Ok(Some(r)) => Some(Self::preview_text(&r)),
            _ => result.subtitle.clone(),
        }
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success { message: None },
            "seed_config" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Run: luma record import --root ~/Documents/Notes/Records --apply"
                        .into(),
                },
            },
            "browse" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "browse is search-driven; use `rec browse CATEGORY`".into(),
                },
            },
            "open" => ActionOutcome::Success {
                message: Some("preview".into()),
            },
            "add" => {
                let payload = action.result.action_payload.as_ref();
                let category = payload
                    .and_then(|p| p.get("category"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        action
                            .result
                            .id
                            .as_str()
                            .strip_prefix("rec:add:")
                            .and_then(|s| s.split(':').next())
                    });
                let name = payload
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        action
                            .result
                            .id
                            .as_str()
                            .strip_prefix("rec:add:")
                            .and_then(|s| s.split(':').nth(1))
                    });
                let (Some(category), Some(name)) = (category, name) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "payload".into(),
                            message: "missing category or name".into(),
                        },
                    };
                };
                let rating = payload
                    .and_then(|p| p.get("rating"))
                    .and_then(|v| v.as_i64())
                    .filter(|r| (1..=10).contains(r));
                let note = payload
                    .and_then(|p| p.get("note"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if let Some(existing) = self
                    .store
                    .get_by_category_and_name(category, name)
                    .ok()
                    .flatten()
                {
                    if !action.confirmation {
                        return ActionOutcome::Failed {
                            kind: FailureKind::InvalidInput {
                                field: "confirmation".into(),
                                message: "overwrite requires confirmation".into(),
                            },
                        };
                    }
                    match self
                        .store
                        .update(existing.id, None, Some(rating), Some(note), None)
                    {
                        Ok(r) => {
                            return ActionOutcome::Success {
                                message: Some(format!("updated {} in {}", r.name, r.category_name)),
                            };
                        }
                        Err(err) => return Self::unavailable_outcome(err),
                    }
                }
                match self.store.insert(category, name, rating, note) {
                    Ok(r) => ActionOutcome::Success {
                        message: Some(format!("added {} to {}", r.name, r.category_name)),
                    },
                    Err(err) => Self::unavailable_outcome(err),
                }
            }
            "rate" => {
                let payload = action.result.action_payload.as_ref();
                let id = payload
                    .and_then(|p| p.get("id"))
                    .and_then(|v| v.as_i64())
                    .or_else(|| Self::parse_record_id(action.result.id.as_str()));
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "id".into(),
                            message: "missing record id".into(),
                        },
                    };
                };
                let record = match Self::get_record_or_outcome(self.store.as_ref(), id) {
                    Ok(r) => r,
                    Err(out) => return out,
                };
                if !action.result.id.as_str().starts_with("rec:rate:") {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "rating".into(),
                            message: format!(
                                "use `rec rate {} SCORE` in prompt (1-10 or clear)",
                                record.id
                            ),
                        },
                    };
                }
                let new_rating: Option<i64> = match payload.and_then(|p| p.get("rating")) {
                    Some(v) if v.is_null() => None,
                    Some(v) => v.as_i64(),
                    None => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::InvalidInput {
                                field: "rating".into(),
                                message: "missing rating in rate command".into(),
                            },
                        };
                    }
                };
                match self.store.set_rating(id, new_rating) {
                    Ok(r) => ActionOutcome::Success {
                        message: Some(format!(
                            "rated {} · {}",
                            r.name,
                            r.rating
                                .map(|x| x.to_string())
                                .unwrap_or_else(|| "cleared".into())
                        )),
                    },
                    Err(err) => Self::unavailable_outcome(err),
                }
            }
            "note" => {
                let payload = action.result.action_payload.as_ref();
                let Some(id) = payload.and_then(|p| p.get("id")).and_then(|v| v.as_i64()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "id".into(),
                            message: "missing record id".into(),
                        },
                    };
                };
                let text = payload
                    .and_then(|p| p.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match self.store.set_note(id, text) {
                    Ok(r) => ActionOutcome::Success {
                        message: Some(format!("updated note for {}", r.name)),
                    },
                    Err(err) => Self::unavailable_outcome(err),
                }
            }
            "remove" => {
                let id = Self::parse_record_id(action.result.id.as_str());
                let Some(id) = id else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "id".into(),
                            message: "expected rec:<id>".into(),
                        },
                    };
                };
                if !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "confirmation".into(),
                            message: "remove requires confirmation".into(),
                        },
                    };
                }
                let record = match Self::get_record_or_outcome(self.store.as_ref(), id) {
                    Ok(r) => r,
                    Err(out) => return out,
                };
                match self.store.delete(id) {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("removed {}", record.name)),
                    },
                    Err(err) => Self::unavailable_outcome(err),
                }
            }
            "import" => {
                let path = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("path"))
                    .and_then(|v| v.as_str())
                    .or_else(|| action.result.id.as_str().strip_prefix("rec:import:"));
                let Some(path) = path else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "path".into(),
                            message: "missing import path".into(),
                        },
                    };
                };
                match self.store.import_dir(std::path::Path::new(path)) {
                    Ok(r) => ActionOutcome::Success {
                        message: Some(format!(
                            "imported {} records ({} skipped) · migration {}",
                            r.inserted,
                            r.skipped,
                            r.migration_id.as_deref().unwrap_or("unknown")
                        )),
                    },
                    Err(err) => Self::unavailable_outcome(err),
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        let root = settings.records_root.as_ref().map(PathBuf::from);
        *self.import_root.write().await = root;
    }

    async fn teardown(&self) {
        *self.store_error.write().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::MemoryRecordsRepository;
    use luma_domain::Query;
    use luma_protocol::Event;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn empty_lists_not_configured() {
        let m = RecordsModule::with_store_for_tests(Arc::new(MemoryRecordsRepository::new()));
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("rec ", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert_eq!(upserts[0].id, "rec:not-configured");
    }

    #[tokio::test]
    async fn search_finds_inserted_record() {
        let store = Arc::new(MemoryRecordsRepository::new());
        store.add_category("电影");
        store.insert("电影", "沙丘", Some(8), "史诗").unwrap();
        let m = RecordsModule::with_store_for_tests(store);
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("rec 沙丘", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("expected chunk");
        };
        assert!(upserts.iter().any(|u| u.title == "沙丘"));
    }
}
