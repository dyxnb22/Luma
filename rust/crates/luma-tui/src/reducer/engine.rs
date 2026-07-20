use crate::effect::Effect;
use crate::view_model::{ActionsIntent, AppState, PendingAction, Route, StatusTone};
use luma_protocol::Event;

use super::actions::{begin_primary_or_confirm, clear_action_ui, execute_action};
use super::preview::preview_effect;
use super::search::begin_search;
use super::{project_remove_name, records_query_active};

/// Orchestrate engine events into local state transitions and follow-up effects.
/// Projection itself remains in `view_model::engine_projection`.
pub(super) fn apply_engine(state: &mut AppState, event: Event) -> Vec<Effect> {
    if let Event::ActionFinished {
        outcome:
            luma_protocol::ActionOutcomeDto::InteractiveTerminal {
                program,
                args,
                record_alias,
            },
        operation_id,
    } = &event
    {
        if state.actions.active_operation.as_deref() == Some(operation_id.as_str()) {
            state
                .status
                .set(format!("starting {program}…"), StatusTone::Progress);
            return vec![Effect::RunInteractiveTerminal {
                program: program.clone(),
                args: args.clone(),
                record_alias: record_alias.clone(),
                operation_id: operation_id.clone(),
            }];
        }
    }
    if let Event::DiagnosticRaised { diagnostic } = &event {
        let settings_conflict =
            diagnostic.get("settings_update").and_then(|v| v.as_str()) == Some("failed");
        if settings_conflict && state.route == Route::Settings {
            let message = diagnostic
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("settings conflict");
            state
                .status
                .set(format!("settings conflict: {message}"), StatusTone::Warning);
            let _ = state.apply_engine_event(event);
            return vec![Effect::GetSettings];
        }
    }
    if let Event::ActionsAvailable { result_id, actions } = event {
        let Some(pending) = state.actions.awaiting_actions.take() else {
            state.status.set(
                format!("{result_id}: {} actions", actions.len()),
                StatusTone::Neutral,
            );
            return vec![Effect::None];
        };
        if pending.result_id != result_id {
            state.actions.awaiting_actions = Some(pending);
            return vec![Effect::None];
        }
        match pending.intent {
            ActionsIntent::Primary => return begin_primary_or_confirm(state, result_id, actions),
            ActionsIntent::RecipeShortcut { action_id } => {
                let resolved = if action_id == "favorite" {
                    actions
                        .iter()
                        .find(|a| a.id == "favorite" || a.id == "unfavorite")
                        .cloned()
                } else {
                    actions.iter().find(|a| a.id == action_id).cloned()
                };
                let Some(action) = resolved else {
                    state.status.set(
                        format!("action `{action_id}` unavailable"),
                        StatusTone::Warning,
                    );
                    return vec![Effect::None];
                };
                if action.needs_confirmation() {
                    state.actions.pending_action = Some(PendingAction {
                        result_id,
                        action: action.clone(),
                    });
                    state.route = Route::ConfirmAction;
                    state.status.set(
                        format!("confirm {}? Enter=yes Esc=no", action.label),
                        StatusTone::Warning,
                    );
                    return vec![Effect::None];
                }
                return execute_action(state, result_id, action, false);
            }
            ActionsIntent::Picker => {
                if actions.is_empty() {
                    state
                        .status
                        .set("no actions available", StatusTone::Warning);
                    return vec![Effect::None];
                }
                state.actions.action_result_id = Some(result_id);
                state.actions.action_choices = actions;
                state.actions.action_selected = 0;
                state.route = Route::ActionPicker;
                state
                    .status
                    .set("pick action · Enter run · Esc back", StatusTone::Neutral);
                return vec![Effect::None];
            }
        }
    }
    let project_remove_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.actions.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && project_remove_name(&state.search.prompt).is_some());
    let records_mutation_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.actions.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && records_query_active(&state.search.prompt));
    let cmd_favorite_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.actions.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(
                outcome,
                luma_protocol::ActionOutcomeDto::Success {
                    message: Some(message),
                    ..
                } if message == "favorited" || message == "unfavorited"
            )
            && super::command_recipes_query_active(&state.search.prompt));
    let refresh_review_stats = matches!(&event, Event::ActionFinished { outcome, .. }
        if matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && matches!(state.route, Route::WordbookReview)
            && state
                .wordbook
                .review
                .as_ref()
                .is_some_and(|r| !r.finished));
    let ready = matches!(event, Event::SessionReady { .. });
    let settings_changed = matches!(event, Event::SettingsChanged { .. });
    let _ = state.apply_engine_event(event);
    if refresh_review_stats {
        return vec![Effect::RefreshWordbookReviewStats];
    }
    if ready {
        let mut effects = vec![Effect::GetSettings, Effect::LoadHub];
        state.schedule_hub_refresh();
        if state.search.results.selected_id.is_some() {
            effects.extend(preview_effect(state));
        }
        return effects;
    }
    if settings_changed {
        state.search.results.items.clear();
        state.search.results.selected_id = None;
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.pending_id = None;
        clear_action_ui(state);
        let mut effects = vec![Effect::LoadHub];
        state.schedule_hub_refresh();
        if !(state.search.prompt.is_empty()
            || (state.actions.active_operation.is_some()
                && project_remove_name(&state.search.prompt).is_some()))
        {
            effects.extend(begin_search(state));
        }
        return effects;
    }
    if project_remove_success {
        if let Some(name) = project_remove_name(&state.search.prompt) {
            state.search.results.items.clear();
            state.search.results.selected_id = None;
            state.preview.body = None;
            state.preview.result_id = None;
            state.preview.pending_id = None;
            state.status.set(
                format!("removed {name} · config only; directory kept"),
                StatusTone::Success,
            );
        }
        return vec![Effect::None];
    }
    if records_mutation_success && !state.search.prompt.trim().is_empty() {
        return begin_search(state);
    }
    if cmd_favorite_success && !state.search.prompt.trim().is_empty() {
        return begin_search(state);
    }
    if let Some(sel) = state.search.results.selected_id.as_deref() {
        let have_body =
            state.preview.result_id.as_deref() == Some(sel) && state.preview.body.is_some();
        let in_flight =
            state.preview.pending_id.is_some() && state.preview.result_id.as_deref() == Some(sel);
        if !have_body && !in_flight {
            return preview_effect(state);
        }
    }
    vec![Effect::None]
}
