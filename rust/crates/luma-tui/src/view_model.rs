use crate::theme::{Symbols, Theme, ThemeMode};
use luma_domain::{FailureKind, SearchItem};
use luma_protocol::{ActionDescriptorDto, ActionOutcomeDto, Event};

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
            theme: Theme::resolve(ThemeMode::Auto),
            symbols: Symbols::detect(),
        }
    }
}

impl AppState {
    pub fn apply_engine_event(&mut self, event: Event) -> bool {
        match event {
            Event::SessionReady => {
                self.status.set("Session ready", StatusTone::Success);
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
                self.active_operation = Some(operation_id.clone());
                self.status.set("Running…", StatusTone::Progress);
                true
            }
            Event::ActionFinished {
                operation_id,
                outcome,
            } => {
                if self.active_operation.as_deref() == Some(operation_id.as_str()) {
                    self.active_operation = None;
                }
                let tone = status_tone_for_outcome(&outcome);
                self.status.set(outcome.display_message(), tone);
                true
            }
            Event::DiagnosticRaised { diagnostic } => {
                self.status
                    .set(format!("doctor: {diagnostic}"), StatusTone::Neutral);
                self.doctor_diagnostic = Some(diagnostic);
                self.route = Route::Doctor;
                true
            }
            Event::SettingsChanged { version, .. } => {
                self.status
                    .set(format!("settings v{version}"), StatusTone::Neutral);
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
}
