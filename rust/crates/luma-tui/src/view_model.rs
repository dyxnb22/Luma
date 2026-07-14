use crate::theme::{Symbols, Theme, ThemeMode};
use luma_domain::{FailureKind, SearchItem};
use luma_protocol::{ActionDescriptorDto, ActionOutcomeDto, Event};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Search,
    Help,
    Doctor,
    Settings,
    Commands,
    QuitConfirm,
    ConfirmAction,
    ActionPicker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionsIntent {
    /// Enter: resolve primary (or first) action, then confirm/execute.
    Primary,
    /// Ctrl-k: show full action picker.
    Picker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitingActions {
    pub intent: ActionsIntent,
    pub result_id: String,
}

#[derive(Clone, Debug)]
pub struct PendingAction {
    pub result_id: String,
    pub action: ActionDescriptorDto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusZone {
    #[default]
    Prompt,
    List,
    Preview,
}

#[derive(Clone, Debug, Default)]
pub struct ResultsView {
    pub items: Vec<SearchItem>,
    pub selected_id: Option<String>,
    /// First visible result index (persisted; render must not invent scroll alone).
    pub scroll: usize,
    /// How many result *rows* fit in the list pane (each result uses 2 terminal lines).
    pub viewport_rows: usize,
}

impl ResultsView {
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.items.iter().position(|i| i.id.as_str() == id))
    }

    pub fn select_at(&mut self, idx: usize) {
        if self.items.is_empty() {
            return;
        }
        let idx = idx.min(self.items.len() - 1);
        self.selected_id = Some(self.items[idx].id.as_str().to_string());
        self.ensure_selection_visible();
    }

    pub fn select_offset(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let idx = self.selected_index().unwrap_or(0) as isize;
        let next = (idx + delta).clamp(0, (self.items.len() - 1) as isize) as usize;
        self.select_at(next);
    }

    pub fn select_next(&mut self) {
        self.select_offset(1);
    }

    pub fn select_prev(&mut self) {
        self.select_offset(-1);
    }

    pub fn ensure_selection_visible(&mut self) {
        let Some(sel) = self.selected_index() else {
            self.scroll = 0;
            return;
        };
        let rows = self.viewport_rows.max(1);
        if sel < self.scroll {
            self.scroll = sel;
        } else if sel >= self.scroll + rows {
            self.scroll = sel + 1 - rows;
        }
        let max_scroll = self.items.len().saturating_sub(rows);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    pub fn set_viewport_rows(&mut self, rows: usize) {
        self.viewport_rows = rows.max(1);
        self.ensure_selection_visible();
    }

    pub fn apply_chunk(&mut self, upserts: Vec<SearchItem>, removed_ids: &[String]) {
        for id in removed_ids {
            self.items.retain(|i| i.id.as_str() != id);
        }
        for item in upserts {
            if let Some(existing) = self.items.iter_mut().find(|i| i.id == item.id) {
                *existing = item;
            } else {
                self.items.push(item);
            }
        }
        self.items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if self.selected_id.is_none() {
            self.selected_id = self.items.first().map(|i| i.id.as_str().to_string());
        } else if let Some(sel) = &self.selected_id {
            if !self.items.iter().any(|i| i.id.as_str() == sel) {
                self.selected_id = self.items.first().map(|i| i.id.as_str().to_string());
            }
        }
        self.ensure_selection_visible();
    }
}

/// Structured status tone — render colors from this, not string parsing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    #[default]
    Neutral,
    Success,
    Progress,
    Warning,
    Error,
    Permission,
}

#[derive(Clone, Debug)]
pub struct StatusLine {
    pub text: String,
    pub tone: StatusTone,
}

impl StatusLine {
    pub fn set(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.text = text.into();
        self.tone = tone;
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            text: "Ready".into(),
            tone: StatusTone::Success,
        }
    }
}

/// Projection for rendering. Not the business source of truth.
#[derive(Clone, Debug)]
pub struct AppState {
    pub route: Route,
    pub prompt: String,
    /// Cursor as a Unicode scalar index into `prompt` (0..=char_count).
    pub prompt_cursor: usize,
    pub active_request: Option<String>,
    pub request_seq_seen: u64,
    pub search_generation: u64,
    pub results: ResultsView,
    pub status: StatusLine,
    pub doctor_diagnostic: Option<serde_json::Value>,
    pub should_quit: bool,
    pub dirty: bool,
    pub awaiting_actions: Option<AwaitingActions>,
    pub pending_action: Option<PendingAction>,
    pub action_choices: Vec<ActionDescriptorDto>,
    /// Result id that produced `action_choices` — never re-read selection on submit.
    pub action_result_id: Option<String>,
    pub action_selected: usize,
    pub active_operation: Option<String>,
    /// Resolved at init / by tests — render must not re-read the environment.
    pub theme: Theme,
    pub symbols: Symbols,
    /// Module id → display_name from SessionReady catalog.
    pub module_labels: HashMap<String, String>,
    /// Full module catalog from SessionReady (workbench metadata).
    pub module_catalog: Vec<ModuleCatalogEntry>,
    /// Pinned Hub rows (clipboard favorites, etc.).
    pub hub_pins: Vec<HubPinRow>,
    /// When set, `FlushSearch` should fire after this Instant.
    pub search_debounce_deadline: Option<std::time::Instant>,
    pub focus: FocusZone,
    /// Newest-first query strings submitted / flushed for search.
    pub query_history: Vec<String>,
    /// When browsing history with Ctrl-p/n; `None` means “live” prompt.
    pub history_browse: Option<usize>,
    /// Previous prompts when drilling into `n browse` / `proj browse` directories.
    /// Esc pops one level; cleared when the prompt is edited or fully cleared.
    pub browse_nav_stack: Vec<String>,
    /// Selected row in empty-state module Hub.
    pub hub_selected: usize,
    /// Settings route: selected row in module list.
    pub settings_selected: usize,
    pub settings_version: u64,
    pub settings_modules: Vec<SettingsModuleRow>,
    /// Doctor overlay scroll (line offset).
    pub doctor_scroll: usize,
    /// Command palette selection.
    pub commands_selected: usize,
    /// Async preview body for the selected result (`LoadPreview`).
    pub preview_result_id: Option<String>,
    pub preview_body: Option<String>,
    /// Line offset when preview pane is focused.
    pub preview_scroll: usize,
    /// Monotonic preview request counter (correlated with `PreviewLoaded.preview_id`).
    pub preview_generation: u64,
    /// In-flight preview request id; `None` when idle.
    pub pending_preview_id: Option<u64>,
    /// After Hub pin Enter, select this result id once search results arrive.
    pub hub_pending_select_id: Option<String>,
    /// First visible Hub row index.
    pub hub_scroll: usize,
    /// Last known terminal size — used to size the results viewport.
    pub term_width: u16,
    pub term_height: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsModuleRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub glyph: Option<String>,
    pub suggested_query: Option<String>,
    pub empty_hint: Option<String>,
    pub supports_browse: bool,
    pub triggers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubPinRow {
    pub id: String,
    pub title: String,
    pub module_id: String,
    pub query: String,
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            route: Route::Search,
            prompt: String::new(),
            prompt_cursor: 0,
            active_request: None,
            request_seq_seen: 0,
            search_generation: 0,
            results: ResultsView::default(),
            status: StatusLine::default(),
            doctor_diagnostic: None,
            should_quit: false,
            dirty: true,
            awaiting_actions: None,
            pending_action: None,
            action_choices: Vec::new(),
            action_result_id: None,
            action_selected: 0,
            active_operation: None,
            theme: Theme::resolve(ThemeMode::Auto),
            symbols: Symbols::detect(),
            module_labels: HashMap::new(),
            module_catalog: Vec::new(),
            hub_pins: Vec::new(),
            search_debounce_deadline: None,
            focus: FocusZone::Prompt,
            query_history: Vec::new(),
            history_browse: None,
            browse_nav_stack: Vec::new(),
            hub_selected: 0,
            settings_selected: 0,
            settings_version: 0,
            settings_modules: Vec::new(),
            doctor_scroll: 0,
            commands_selected: 0,
            preview_result_id: None,
            preview_body: None,
            preview_scroll: 0,
            preview_generation: 0,
            pending_preview_id: None,
            hub_pending_select_id: None,
            hub_scroll: 0,
            term_width: 80,
            term_height: 24,
        };
        state.sync_results_viewport();
        state
    }
}

impl AppState {
    pub fn prompt_char_len(&self) -> usize {
        self.prompt.chars().count()
    }

    pub fn clamp_prompt_cursor(&mut self) {
        let len = self.prompt_char_len();
        if self.prompt_cursor > len {
            self.prompt_cursor = len;
        }
    }

    fn prompt_byte_index(&self, char_idx: usize) -> usize {
        self.prompt
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.prompt.len())
    }

    pub fn insert_prompt_char(&mut self, c: char) {
        self.clamp_prompt_cursor();
        let byte = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.insert(byte, c);
        self.prompt_cursor += 1;
    }

    pub fn backspace_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor == 0 {
            return;
        }
        let del_at = self.prompt_cursor - 1;
        let start = self.prompt_byte_index(del_at);
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(start..end, "");
        self.prompt_cursor = del_at;
    }

    pub fn delete_forward_prompt(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor >= self.prompt_char_len() {
            return;
        }
        let start = self.prompt_byte_index(self.prompt_cursor);
        let end = self.prompt_byte_index(self.prompt_cursor + 1);
        self.prompt.replace_range(start..end, "");
    }

    pub fn clear_prompt(&mut self) {
        self.prompt.clear();
        self.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-u: delete from start through character before cursor.
    pub fn clear_prompt_to_start(&mut self) {
        self.clamp_prompt_cursor();
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(0..end, "");
        self.prompt_cursor = 0;
    }

    /// Readline-style Ctrl-w: delete the word before the cursor.
    pub fn delete_prompt_word_back(&mut self) {
        self.clamp_prompt_cursor();
        if self.prompt_cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.prompt.chars().collect();
        let mut i = self.prompt_cursor;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        let start = self.prompt_byte_index(i);
        let end = self.prompt_byte_index(self.prompt_cursor);
        self.prompt.replace_range(start..end, "");
        self.prompt_cursor = i;
    }

    pub fn preview_side_by_side(&self) -> bool {
        self.term_width >= 100 && !self.results.items.is_empty()
    }

    /// Stacked preview under results for tall narrow terminals (keeps 80×24 list-only).
    pub fn preview_stacked(&self) -> bool {
        self.term_width < 100
            && self.term_width >= 60
            && self.term_height >= 28
            && !self.results.items.is_empty()
    }

    pub fn preview_visible(&self) -> bool {
        self.preview_side_by_side() || self.preview_stacked()
    }

    pub fn sync_results_viewport(&mut self) {
        // Vertical chrome: prompt(3) + status(3); list borders(2); each row = 2 lines.
        let body = self.term_height.saturating_sub(6) as usize;
        let preview_take = if self.preview_stacked() { 8 } else { 0 };
        let list_inner = body.saturating_sub(2 + preview_take);
        let rows = (list_inner / 2).max(1);
        self.results.set_viewport_rows(rows);
        self.ensure_hub_selection_visible();
    }

    pub fn ensure_hub_selection_visible(&mut self) {
        let len = self.hub_rows().len();
        if len == 0 {
            self.hub_scroll = 0;
            self.hub_selected = 0;
            return;
        }
        if self.hub_selected >= len {
            self.hub_selected = len - 1;
        }
        let rows = self.results.viewport_rows.max(1);
        if self.hub_selected < self.hub_scroll {
            self.hub_scroll = self.hub_selected;
        } else if self.hub_selected >= self.hub_scroll + rows {
            self.hub_scroll = self.hub_selected + 1 - rows;
        }
        let max_scroll = len.saturating_sub(rows);
        if self.hub_scroll > max_scroll {
            self.hub_scroll = max_scroll;
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusZone::Prompt => FocusZone::List,
            FocusZone::List if self.preview_visible() => FocusZone::Preview,
            FocusZone::List => FocusZone::Prompt,
            FocusZone::Preview => FocusZone::Prompt,
        };
    }

    pub fn push_query_history(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        self.query_history.retain(|h| h != q);
        self.query_history.insert(0, q.to_string());
        self.query_history.truncate(50);
        self.history_browse = None;
    }

    pub fn history_older(&mut self) {
        if self.query_history.is_empty() {
            return;
        }
        let next = match self.history_browse {
            None => 0,
            Some(i) => (i + 1).min(self.query_history.len() - 1),
        };
        self.history_browse = Some(next);
        self.browse_nav_stack.clear();
        self.prompt = self.query_history[next].clone();
        self.prompt_cursor = self.prompt_char_len();
    }

    pub fn history_newer(&mut self) {
        let Some(i) = self.history_browse else {
            return;
        };
        if i == 0 {
            self.history_browse = None;
            self.browse_nav_stack.clear();
            self.clear_prompt();
            return;
        }
        let next = i - 1;
        self.history_browse = Some(next);
        self.browse_nav_stack.clear();
        self.prompt = self.query_history[next].clone();
        self.prompt_cursor = self.prompt_char_len();
    }

    /// Sorted hub rows: modules then metadata-driven suggested queries.
    /// Returns (kind, id, title, query_or_empty) where kind is "pin" | "module".
    pub fn hub_rows(&self) -> Vec<(String, String, String, String)> {
        let mut rows = Vec::new();
        for pin in &self.hub_pins {
            rows.push((
                "pin".into(),
                pin.id.clone(),
                pin.title.clone(),
                pin.query.clone(),
            ));
        }
        let mut modules: Vec<_> = self
            .module_catalog
            .iter()
            .filter(|m| m.enabled)
            .cloned()
            .collect();
        if modules.is_empty() {
            // Fallback while SessionReady catalog is empty.
            modules = self
                .module_labels
                .iter()
                .map(|(id, name)| ModuleCatalogEntry {
                    id: id.clone(),
                    display_name: name.clone(),
                    enabled: true,
                    glyph: None,
                    suggested_query: None,
                    empty_hint: None,
                    supports_browse: false,
                    triggers: vec![],
                })
                .collect();
        }
        modules.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
        for m in modules {
            let query = m.suggested_query.clone().unwrap_or_else(|| {
                let key =
                    m.id.strip_prefix("luma.")
                        .unwrap_or(m.id.as_str())
                        .split('.')
                        .next()
                        .unwrap_or(m.id.as_str());
                let trigger = match key {
                    "apps" => "app",
                    "clipboard" => "clip",
                    "notes" => "n",
                    "quicklinks" => "ql",
                    "snippets" => "s",
                    "projects" => "proj",
                    other => other,
                };
                format!("{trigger} ")
            });
            rows.push(("module".into(), m.id.clone(), m.display_name.clone(), query));
        }
        rows
    }

    fn try_select_hub_pending(&mut self) {
        let Some(want) = self.hub_pending_select_id.clone() else {
            return;
        };
        if self.results.items.iter().any(|i| i.id.as_str() == want) {
            self.results.selected_id = Some(want);
            self.results.ensure_selection_visible();
            self.hub_pending_select_id = None;
            self.preview_body = None;
            self.preview_result_id = None;
            self.pending_preview_id = None;
        }
    }

    /// Sorted hub rows: (module_id, display_name, suggested_query).
    pub fn hub_entries(&self) -> Vec<(String, String, String)> {
        self.hub_rows()
            .into_iter()
            .filter(|(kind, _, _, _)| kind == "module")
            .map(|(_, id, name, query)| (id, name, query))
            .collect()
    }

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
            Event::HubLoaded { pins } => {
                self.hub_pins = pins
                    .into_iter()
                    .map(|p| HubPinRow {
                        id: p.id,
                        title: p.title,
                        module_id: p.module_id,
                        query: p.query,
                    })
                    .collect();
                true
            }
            Event::SearchStarted { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.set("Searching…", StatusTone::Progress);
                    true
                } else {
                    false
                }
            }
            Event::ResultsReset { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.results.items.clear();
                    self.results.selected_id = None;
                    self.request_seq_seen = 0;
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
                if self.active_request.as_deref() != Some(request_id.as_str()) {
                    return false;
                }
                if sequence <= self.request_seq_seen {
                    return false;
                }
                self.request_seq_seen = sequence;
                let items: Vec<_> = upserts.into_iter().map(|d| d.into_domain()).collect();
                self.results.apply_chunk(items, &removed_ids);
                self.try_select_hub_pending();
                true
            }
            Event::SearchFinished {
                request_id,
                total,
                elapsed_ms: _,
            } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    // End the active request so Esc Clear works on the first press.
                    self.active_request = None;
                    self.try_select_hub_pending();
                    let (text, tone) = if total == 0 {
                        ("No results".into(), StatusTone::Neutral)
                    } else if total == 1 {
                        ("1 result".into(), StatusTone::Success)
                    } else {
                        (format!("{total} results"), StatusTone::Success)
                    };
                    self.status.set(text, tone);
                    true
                } else {
                    false
                }
            }
            Event::SearchCancelled { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.active_request = None;
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
                if let Some(active) = self.active_operation.as_deref() {
                    if active != operation_id.as_str() {
                        // Concurrent op the TUI is not tracking — ignore.
                        return false;
                    }
                }
                self.active_operation = Some(operation_id.clone());
                self.status.set("Running…", StatusTone::Progress);
                true
            }
            Event::ActionFinished {
                operation_id,
                outcome,
            } => {
                if let Some(active) = self.active_operation.as_deref() {
                    if active != operation_id.as_str() {
                        // Late finish for a non-current operation.
                        return false;
                    }
                    self.active_operation = None;
                }
                let tone = status_tone_for_outcome(&outcome);
                self.status.set(outcome.display_message(), tone);
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
                self.status
                    .set(format!("doctor: {diagnostic}"), StatusTone::Neutral);
                self.doctor_diagnostic = Some(diagnostic);
                self.doctor_scroll = 0;
                if !matches!(self.route, Route::Settings | Route::Commands) {
                    self.route = Route::Doctor;
                }
                true
            }
            Event::SettingsChanged { version, settings } => {
                self.settings_version = version;
                self.settings_modules.clear();
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
                        self.settings_modules
                            .push(SettingsModuleRow { id, name, enabled });
                    }
                }
                if self.settings_selected >= self.settings_modules.len()
                    && !self.settings_modules.is_empty()
                {
                    self.settings_selected = self.settings_modules.len() - 1;
                }
                self.module_labels = self
                    .settings_modules
                    .iter()
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect();
                for row in &self.settings_modules {
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
                if self.pending_preview_id != Some(preview_id) {
                    return false;
                }
                if self.results.selected_id.as_deref() != Some(result_id.as_str()) {
                    return false;
                }
                self.pending_preview_id = None;
                self.preview_result_id = Some(result_id);
                self.preview_body = Some(body);
                self.preview_scroll = 0;
                true
            }
            Event::ActionsAvailable { result_id, actions } => {
                if self.awaiting_actions.is_none() {
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
            _ => false,
        }
    }
}

fn status_tone_for_outcome(outcome: &ActionOutcomeDto) -> StatusTone {
    match outcome {
        ActionOutcomeDto::Success { .. } => StatusTone::Success,
        ActionOutcomeDto::Cancelled => StatusTone::Warning,
        ActionOutcomeDto::Failed { kind, .. } => status_tone_for_failure(kind),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_finished_cancelled_is_warning() {
        let mut state = AppState {
            active_operation: Some("op-1".into()),
            ..AppState::default()
        };
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Cancelled,
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.active_operation.is_none());
    }

    #[test]
    fn stale_action_finished_does_not_overwrite_status() {
        let mut state = AppState {
            active_operation: Some("op-current".into()),
            ..AppState::default()
        };
        state.status.set("running current", StatusTone::Progress);
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-old".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("stale ok".into()),
            },
        });
        assert!(!applied);
        assert_eq!(state.active_operation.as_deref(), Some("op-current"));
        assert_eq!(state.status.text, "running current");
    }

    #[test]
    fn action_finished_not_configured_is_warning() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-2".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::NotConfigured {
                remediation: "set notes_root".into(),
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.status.text.contains("not_configured"));
    }

    #[test]
    fn action_finished_unavailable_is_warning() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-3".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::Unavailable {
                reason: "signed host required".into(),
                retryable: false,
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Warning);
        assert!(state.status.text.contains("unavailable"));
    }

    #[test]
    fn action_finished_permission_is_permission_tone() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-4".into(),
            outcome: ActionOutcomeDto::failed(FailureKind::PermissionRequired {
                capability: "accessibility".into(),
                guidance: "Open Settings".into(),
            }),
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Permission);
    }

    #[test]
    fn action_finished_success_is_success() {
        let mut state = AppState::default();
        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-5".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("Opened Safari".into()),
            },
        });
        assert!(applied);
        assert_eq!(state.status.tone, StatusTone::Success);
    }

    #[test]
    fn preview_stacked_on_tall_narrow_terminal() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};

        let mut state = AppState {
            term_width: 80,
            term_height: 28,
            ..AppState::default()
        };
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("luma.notes"),
            title: "Note".into(),
            subtitle: None,
            kind: "note".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
        });
        assert!(!state.preview_side_by_side());
        assert!(state.preview_stacked());
        assert!(state.preview_visible());

        state.term_height = 24;
        assert!(!state.preview_stacked());
        assert!(!state.preview_visible());
    }
}
