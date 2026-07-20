use crate::effect::Effect;
use crate::view_model::{AppState, StatusTone};

use super::actions::clear_action_ui;

pub(super) fn schedule_search(state: &mut AppState) -> Vec<Effect> {
    // Cancel in-flight work immediately so typing stays responsive, but delay the
    // new Search until the quiet period so bursts don't thrash modules.
    let mut effects = cancel_active(state);
    // Keep prior results visible during debounce (clear only in begin_search).
    clear_action_ui(state);
    if state.search.prompt.is_empty() {
        state.search.debounce_deadline = None;
        state.search.results.items.clear();
        state.search.results.selected_id = None;
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.pending_id = None;
        state.status.set("Ready", StatusTone::Neutral);
        state.schedule_hub_refresh();
        effects.push(Effect::LoadHub);
        return effects;
    }
    state.hub.refresh_deadline = None;
    state.search.debounce_deadline =
        Some(std::time::Instant::now() + std::time::Duration::from_millis(80));
    state.status.set("Typing…", StatusTone::Progress);
    effects
}

pub(super) fn flush_pending_search_or_continue(state: &mut AppState) -> Option<Vec<Effect>> {
    if state.search.debounce_deadline.is_some() {
        state.search.debounce_deadline = None;
        return Some(begin_search(state));
    }
    None
}

pub(super) fn begin_search(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.search.debounce_deadline = None;
    let mut effects = cancel_active(state);
    if state.search.prompt.is_empty() {
        state.search.results.items.clear();
        state.search.results.selected_id = None;
        state.search.results.scroll = 0;
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.pending_id = None;
        state.status.set("Ready", StatusTone::Neutral);
        state.schedule_hub_refresh();
        effects.push(Effect::LoadHub);
        return effects;
    }
    state.hub.refresh_deadline = None;
    state.push_query_history(&state.search.prompt.clone());
    let request_id = next_request_id(state);
    state.search.active_request = Some(request_id.clone());
    state.search.request_seq_seen = 0;
    state.search.results.items.clear();
    state.search.results.selected_id = None;
    // New search invalidates any in-flight preview for prior rows.
    state.preview.body = None;
    state.preview.result_id = None;
    state.preview.pending_id = None;
    effects.push(Effect::Search {
        request_id,
        query: state.search.prompt.clone(),
    });
    effects
}

pub(super) fn cancel_active(state: &mut AppState) -> Vec<Effect> {
    if let Some(request_id) = state.search.active_request.take() {
        vec![Effect::CancelSearch { request_id }]
    } else {
        Vec::new()
    }
}

pub(super) fn next_request_id(state: &mut AppState) -> String {
    state.search.search_generation = state.search.search_generation.saturating_add(1);
    format!("req-{}", state.search.search_generation)
}
