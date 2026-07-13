use luma_domain::SearchItem;
use luma_protocol::{ActionDescriptorDto, Event};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Search,
    Help,
    Doctor,
    QuitConfirm,
    ConfirmAction,
    ActionPicker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionsIntent {
    /// Enter: resolve primary (or first) action, then confirm/execute.
    Primary,
    /// Tab: show full action picker.
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

#[derive(Clone, Debug, Default)]
pub struct ResultsView {
    pub items: Vec<SearchItem>,
    pub selected_id: Option<String>,
}

impl ResultsView {
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let idx = self
            .selected_id
            .as_ref()
            .and_then(|id| self.items.iter().position(|i| i.id.as_str() == id))
            .unwrap_or(0);
        let next = (idx + 1).min(self.items.len() - 1);
        self.selected_id = Some(self.items[next].id.as_str().to_string());
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let idx = self
            .selected_id
            .as_ref()
            .and_then(|id| self.items.iter().position(|i| i.id.as_str() == id))
            .unwrap_or(0);
        let prev = idx.saturating_sub(1);
        self.selected_id = Some(self.items[prev].id.as_str().to_string());
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
    }
}

#[derive(Clone, Debug)]
pub struct StatusLine {
    pub text: String,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            text: "ready".into(),
        }
    }
}

/// Projection for rendering. Not the business source of truth.
#[derive(Clone, Debug)]
pub struct AppState {
    pub route: Route,
    pub prompt: String,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            route: Route::Search,
            prompt: String::new(),
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
        }
    }
}

impl AppState {
    pub fn apply_engine_event(&mut self, event: Event) -> bool {
        match event {
            Event::SessionReady => {
                self.status.text = "session ready".into();
                true
            }
            Event::SearchStarted { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.text = format!("searching {request_id}");
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
                true
            }
            Event::SearchFinished {
                request_id,
                total,
                elapsed_ms,
            } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.text = format!("done {total} in {elapsed_ms}ms");
                    true
                } else {
                    false
                }
            }
            Event::SearchCancelled { request_id } => {
                if self.active_request.as_deref() == Some(request_id.as_str()) {
                    self.status.text = format!("cancelled {request_id}");
                    true
                } else {
                    false
                }
            }
            Event::Fatal {
                correlation_id,
                message,
            } => {
                self.status.text = format!("fatal [{correlation_id}]: {message}");
                true
            }
            Event::ActionStarted { operation_id } => {
                self.active_operation = Some(operation_id.clone());
                self.status.text = format!("action {operation_id}…");
                true
            }
            Event::ActionFinished {
                operation_id,
                outcome,
            } => {
                if self.active_operation.as_deref() == Some(operation_id.as_str()) {
                    self.active_operation = None;
                }
                self.status.text = outcome.display_message();
                true
            }
            Event::DiagnosticRaised { diagnostic } => {
                self.status.text = format!("doctor: {diagnostic}");
                self.doctor_diagnostic = Some(diagnostic);
                self.route = Route::Doctor;
                true
            }
            Event::SettingsChanged { version, .. } => {
                self.status.text = format!("settings v{version}");
                true
            }
            Event::ActionsAvailable { result_id, actions } => {
                if self.awaiting_actions.is_none() {
                    self.status.text = format!("{result_id}: {} actions", actions.len());
                    return true;
                }
                // Handled by reducer via Msg::Engine so effects can be returned.
                self.status.text = format!("{result_id}: {} actions", actions.len());
                true
            }
            _ => false,
        }
    }
}
