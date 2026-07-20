use crate::view_model::{
    AppState, HubWindowRow, HubWindowsState, ModuleCatalogEntry, Route, SettingsModuleRow,
    StatusTone, WordbookReviewState, WordbookReviewStats, WordbookReviewWord,
};
use luma_domain::FailureKind;
use luma_protocol::{ActionOutcomeDto, Event};

impl AppState {
    pub fn apply_engine_event(&mut self, event: Event) -> bool {
        match event {
            Event::SessionReady { modules } => {
                self.module_catalog = modules
                    .iter()
                    .map(|m| ModuleCatalogEntry {
                        id: m.id.clone(),
                        display_name: m.display_name.clone(),
                        enabled: m.enabled,
                        glyph: m.glyph.clone(),
                        suggested_query: m.suggested_query.clone(),
                        empty_hint: m.empty_hint.clone(),
                        supports_browse: m.supports_browse,
                        triggers: m.triggers.clone(),
                    })
                    .collect();
                self.module_labels = modules
                    .into_iter()
                    .map(|m| (m.id, m.display_name))
                    .collect();
                self.status.set("Session ready", StatusTone::Success);
                true
            }
            Event::HubLoaded { windows } => {
                self.hub.windows = windows.map(|w| HubWindowsState {
                    app_name: w.app_name,
                    windows: w
                        .windows
                        .into_iter()
                        .map(|row| HubWindowRow {
                            id: row.id,
                            title: row.title,
                        })
                        .collect(),
                    more: w.more,
                    status_kind: w.status.as_ref().map(|s| s.kind.clone()),
                    status_title: w.status.as_ref().map(|s| s.title.clone()),
                    status_subtitle: w.status.and_then(|s| s.subtitle),
                });
                self.ensure_hub_selection_visible();
                self.schedule_hub_refresh();
                true
            }
            Event::SnapshotLoaded {
                items,
                module_states: _,
            } => {
                // A newer search (or debounce) owns the list — ignore lag resync.
                if self.search.active_request.is_some() || self.search.debounce_deadline.is_some() {
                    return false;
                }
                let mut items: Vec<_> = items.into_iter().map(|d| d.into_domain()).collect();
                items.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.id.as_str().cmp(b.id.as_str()))
                });
                self.search.results.items = items;
                self.search.results.selected_id = self
                    .search
                    .results
                    .items
                    .first()
                    .map(|i| i.id.as_str().to_string());
                self.search.results.scroll = 0;
                self.sync_results_viewport();
                self.status
                    .set("Resynced after lag", crate::view_model::StatusTone::Warning);
                true
            }
            Event::SearchStarted { request_id } => {
                if self.search.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.set("Searching…", StatusTone::Progress);
                    true
                } else {
                    false
                }
            }
            Event::ResultsReset { request_id } => {
                if self.search.active_request.as_deref() == Some(request_id.as_str()) {
                    self.search.results.items.clear();
                    self.search.results.selected_id = None;
                    self.search.request_seq_seen = 0;
                    true
                } else {
                    false
                }
            }
            Event::ResultsChunk {
                request_id,
                sequence,
                upserts,
                removed_ids,
            } => {
                // Empty request_id: module-disable eviction (engine purge). Apply removals
                // regardless of the active search so disabled-module rows leave the UI.
                if request_id.is_empty() {
                    if removed_ids.is_empty() {
                        return false;
                    }
                    self.search.results.apply_chunk(Vec::new(), &removed_ids);
                    return true;
                }
                if self.search.active_request.as_deref() != Some(request_id.as_str()) {
                    return false;
                }
                if sequence <= self.search.request_seq_seen {
                    return false;
                }
                self.search.request_seq_seen = sequence;
                let items: Vec<_> = upserts.into_iter().map(|d| d.into_domain()).collect();
                self.search.results.apply_chunk(items, &removed_ids);
                true
            }
            Event::SearchFinished {
                request_id,
                total,
                elapsed_ms,
            } => {
                if self.search.active_request.as_deref() == Some(request_id.as_str()) {
                    // End the active request so Esc Clear works on the first press.
                    self.search.active_request = None;
                    let (text, tone) = if self.incomplete_slash_trigger().is_some() {
                        ("Add space to search".into(), StatusTone::Neutral)
                    } else if total == 0 {
                        ("No results".into(), StatusTone::Neutral)
                    } else {
                        (format!("{elapsed_ms}ms"), StatusTone::Success)
                    };
                    self.status.set(text, tone);
                    true
                } else {
                    false
                }
            }
            Event::SearchCancelled { request_id } => {
                if self.search.active_request.as_deref() == Some(request_id.as_str()) {
                    self.search.active_request = None;
                    self.status.set("Search cancelled", StatusTone::Warning);
                    true
                } else {
                    false
                }
            }
            Event::Fatal {
                correlation_id: _,
                message,
            } => {
                self.status
                    .set(format!("Error: {message}"), StatusTone::Error);
                true
            }
            Event::ActionStarted { operation_id } => {
                if self.actions.active_operation.as_deref() != Some(operation_id.as_str()) {
                    return false;
                }
                self.status.set("Running…", StatusTone::Progress);
                true
            }
            Event::WordbookReviewLoaded {
                queue,
                words,
                stats,
            } => {
                if !matches!(self.route, Route::WordbookReview) {
                    return false;
                }
                let word_items = words
                    .into_iter()
                    .map(|w| WordbookReviewWord {
                        id: w.id,
                        term: w.term,
                        phonetic: w.phonetic,
                        meaning: w.meaning,
                        example: w.example,
                    })
                    .collect::<Vec<_>>();
                let finished = word_items.is_empty();
                self.wordbook.review = Some(WordbookReviewState {
                    words: word_items,
                    index: 0,
                    revealed: false,
                    stats: WordbookReviewStats {
                        queue,
                        due: stats.due,
                        new_count: stats.new_count,
                        wrong: stats.wrong,
                        goal: stats.goal,
                        reviewed_today: stats.reviewed_today,
                        remaining_goal: stats.remaining_goal,
                        ..WordbookReviewStats::default()
                    },
                    finished,
                    pending_grade: None,
                });
                if finished {
                    self.status.set(
                        "review queue empty · try /wb review new",
                        StatusTone::Warning,
                    );
                } else {
                    self.status
                        .set("review · Enter reveal · 1/2/3 grade", StatusTone::Neutral);
                }
                true
            }
            Event::ActionFinished {
                operation_id,
                outcome,
            } => {
                if self.actions.active_operation.as_deref() != Some(operation_id.as_str()) {
                    return false;
                }
                self.actions.active_operation = None;
                if let luma_protocol::ActionOutcomeDto::InteractiveRecipeRun { plan } = &outcome {
                    self.runtime.pending_recipe_run = Some((**plan).clone());
                    self.status
                        .set("recipe ready — running in terminal…", StatusTone::Progress);
                    return true;
                }
                if matches!(self.route, Route::WordbookReview) {
                    if matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. }) {
                        if let Some(review) = self.wordbook.review.as_mut() {
                            if let Some(action) = review.pending_grade.take() {
                                match action.as_str() {
                                    "known" => review.stats.session_known += 1,
                                    "fuzzy" => review.stats.session_fuzzy += 1,
                                    "unknown" => review.stats.session_unknown += 1,
                                    "mastered" => review.stats.session_mastered += 1,
                                    _ => {}
                                }
                            }
                            review.revealed = false;
                            review.index += 1;
                            if review.index >= review.words.len() {
                                review.finished = true;
                            }
                        }
                    } else if let Some(review) = self.wordbook.review.as_mut() {
                        // A cancelled grade must not remain armed for the next keypress.
                        review.pending_grade = None;
                    }
                    let tone = status_tone_for_outcome(&outcome);
                    if self.wordbook.review.as_ref().is_some_and(|r| r.finished) {
                        if let Some(review) = &self.wordbook.review {
                            self.status.set(
                                format!(
                                    "review done · K{} F{} U{} · goal {} · reviewed {}",
                                    review.stats.session_known,
                                    review.stats.session_fuzzy,
                                    review.stats.session_unknown,
                                    review.stats.goal,
                                    review.stats.reviewed_today
                                ),
                                StatusTone::Success,
                            );
                        }
                    } else {
                        self.status.set(outcome.user_message(), tone);
                    }
                    return true;
                }
                let tone = status_tone_for_outcome(&outcome);
                self.status.set(outcome.user_message(), tone);
                true
            }
            Event::WordbookReviewStatsUpdated { stats } => {
                if let Some(review) = self.wordbook.review.as_mut() {
                    review.stats.due = stats.due;
                    review.stats.new_count = stats.new_count;
                    review.stats.wrong = stats.wrong;
                    review.stats.goal = stats.goal;
                    review.stats.reviewed_today = stats.reviewed_today;
                    review.stats.remaining_goal = stats.remaining_goal;
                }
                true
            }
            Event::DiagnosticRaised { diagnostic } => {
                let settings_conflict =
                    diagnostic.get("settings_update").and_then(|v| v.as_str()) == Some("failed");
                if settings_conflict {
                    let message = diagnostic
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("settings update failed");
                    self.status
                        .set(format!("settings conflict: {message}"), StatusTone::Warning);
                    return true;
                }
                false
            }
            Event::SettingsChanged { version, settings } => {
                self.settings.version = version;
                self.settings.modules.clear();
                self.settings.roots.notes_root = settings
                    .get("notes_root")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                self.settings.roots.projects_roots = settings
                    .get("projects_roots")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                self.settings.roots.imported_projects = settings
                    .get("imported_projects")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.get("path").and_then(|p| p.as_str()).map(str::to_string)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.settings.roots.loaded = true;
                if let Some(modules) = settings.get("modules").and_then(|v| v.as_array()) {
                    for row in modules {
                        let id = row
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if id.is_empty() {
                            continue;
                        }
                        let name = row
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(id.as_str())
                            .to_string();
                        let enabled = row.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                        self.settings
                            .modules
                            .push(SettingsModuleRow { id, name, enabled });
                    }
                }
                if self.settings.selected >= self.settings.modules.len()
                    && !self.settings.modules.is_empty()
                {
                    self.settings.selected = self.settings.modules.len() - 1;
                }
                self.module_labels = self
                    .settings
                    .modules
                    .iter()
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect();
                for row in &self.settings.modules {
                    if let Some(entry) = self.module_catalog.iter_mut().find(|m| m.id == row.id) {
                        entry.enabled = row.enabled;
                    }
                }
                self.status
                    .set(format!("settings v{version}"), StatusTone::Neutral);
                true
            }
            Event::PreviewLoaded {
                result_id,
                preview_id,
                body,
            } => {
                if self.preview.pending_id != Some(preview_id) {
                    return false;
                }
                if self.search.results.selected_id.as_deref() != Some(result_id.as_str()) {
                    // Matching generation but wrong selection — clear so preview can retry.
                    self.preview.pending_id = None;
                    return false;
                }
                self.preview.pending_id = None;
                self.preview.result_id = Some(result_id);
                self.preview.body = Some(body);
                self.preview.scroll = 0;
                true
            }
            Event::ActionsAvailable { result_id, actions } => {
                if self.actions.awaiting_actions.is_none() {
                    self.status.set(
                        format!("{result_id}: {} actions", actions.len()),
                        StatusTone::Neutral,
                    );
                    return true;
                }
                self.status.set(
                    format!("{result_id}: {} actions", actions.len()),
                    StatusTone::Neutral,
                );
                true
            }
            Event::ModuleStateChanged { module_id, state } => {
                let enabled = state != "disabled";
                if let Some(entry) = self.module_catalog.iter_mut().find(|m| m.id == module_id) {
                    entry.enabled = enabled;
                }
                if let Some(row) = self.settings.modules.iter_mut().find(|m| m.id == module_id) {
                    row.enabled = enabled;
                }
                true
            }
        }
    }
}

fn status_tone_for_outcome(outcome: &ActionOutcomeDto) -> StatusTone {
    match outcome {
        ActionOutcomeDto::Success { .. } => StatusTone::Success,
        ActionOutcomeDto::Cancelled => StatusTone::Warning,
        ActionOutcomeDto::Failed { kind, .. } => status_tone_for_failure(kind),
        ActionOutcomeDto::InteractiveRecipeRun { .. } => StatusTone::Progress,
        ActionOutcomeDto::InteractiveTerminal { .. } => StatusTone::Progress,
    }
}

fn status_tone_for_failure(kind: &FailureKind) -> StatusTone {
    match kind {
        FailureKind::PermissionRequired { .. } => StatusTone::Permission,
        FailureKind::Warming { .. } => StatusTone::Progress,
        FailureKind::Cancelled => StatusTone::Warning,
        FailureKind::NotConfigured { .. } | FailureKind::Unavailable { .. } => StatusTone::Warning,
        FailureKind::Timeout { .. }
        | FailureKind::InvalidInput { .. }
        | FailureKind::NotFound { .. }
        | FailureKind::Conflict { .. }
        | FailureKind::SecurityDenied { .. }
        | FailureKind::Io { .. }
        | FailureKind::Internal { .. } => StatusTone::Error,
    }
}
