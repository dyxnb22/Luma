use crate::effect::Effect;
use crate::view_model::{
    ActionsIntent, AppState, AwaitingActions, PendingAction, Route, StatusTone,
};
use luma_protocol::ActionDescriptorDto;

use super::apply_ui_intent;
use super::navigation::{
    drill_into_browse, open_notes_issues, seed_module_add, seed_module_config, seed_record_edit,
};
use super::next_operation_id;
use super::resolve_ui_intent;
use super::wordbook;

pub(super) fn clear_action_ui(state: &mut AppState) {
    state.actions.awaiting_actions = None;
    state.actions.pending_action = None;
    state.actions.action_choices.clear();
    state.actions.action_result_id = None;
    state.actions.action_selected = 0;
}

pub(super) fn dismiss_help_for_prompt_edit(state: &mut AppState) {
    state.overlay.restore_prompt = None;
    state.route = Route::Search;
}

pub(super) fn recipe_shortcut(state: &mut AppState, action_id: &str) -> Vec<Effect> {
    let Some(item) = state.selected_search_item().cloned() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    if item.module_id.as_str() != "luma.command_recipes" {
        return vec![Effect::None];
    }
    let result_id = item.id.as_str().to_string();
    state.actions.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::RecipeShortcut {
            action_id: action_id.to_string(),
        },
        result_id: result_id.clone(),
    });
    state
        .status
        .set(format!("resolving {action_id}…"), StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

pub(super) fn request_primary_actions(state: &mut AppState) -> Vec<Effect> {
    let Some(item) = state
        .search
        .results
        .selected_id
        .as_ref()
        .and_then(|id| {
            state
                .search
                .results
                .items
                .iter()
                .find(|i| i.id.as_str() == id.as_str())
        })
        .cloned()
    else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    if let Some(queue) = wordbook::wordbook_review_queue_from_item(&item) {
        return wordbook::begin_wordbook_review(state, queue);
    }
    if let Some(intent) = resolve_ui_intent(&item) {
        return apply_ui_intent(state, &item, intent);
    }
    let result_id = item.id.as_str().to_string();
    state.actions.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Primary,
        result_id: result_id.clone(),
    });
    state.status.set("resolving actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

pub(super) fn request_action_picker(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Search {
        return vec![Effect::None];
    }
    let Some(result_id) = state.search.results.selected_id.clone() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    state.actions.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Picker,
        result_id: result_id.clone(),
    });
    state.status.set("loading actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

pub(super) fn review_return_route(state: &AppState) -> Route {
    if state.wordbook.review.is_some() {
        Route::WordbookReview
    } else {
        Route::Search
    }
}

pub(super) fn confirm_pending(state: &mut AppState) -> Vec<Effect> {
    let Some(pending) = state.actions.pending_action.take() else {
        state.route = review_return_route(state);
        return vec![Effect::None];
    };
    state.route = review_return_route(state);
    execute_action(state, pending.result_id, pending.action, true)
}

pub(super) fn submit_picker_selection(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.actions.action_result_id.take() else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    let Some(action) = state
        .actions
        .action_choices
        .get(state.actions.action_selected)
        .cloned()
    else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    state.actions.action_choices.clear();
    state.actions.action_selected = 0;
    if matches!(action.id.as_str(), "rate" | "note") {
        if let Some(item) = state
            .search
            .results
            .items
            .iter()
            .find(|i| i.id.as_str() == result_id.as_str() && i.kind == "record")
            .cloned()
        {
            state.route = Route::Search;
            return seed_record_edit(state, &item, action.id.as_str());
        }
    }
    if action.id == "browse" {
        state.route = Route::Search;
        if let Some(item) = state
            .search
            .results
            .items
            .iter()
            .find(|i| i.id.as_str() == result_id.as_str())
            .cloned()
        {
            return drill_into_browse(state, &item);
        }
        return vec![Effect::None];
    }
    if action.id == "list_issues" {
        state.route = Route::Search;
        return open_notes_issues(state);
    }
    if action.id == "seed_add" {
        state.route = Route::Search;
        if let Some(item) = state
            .search
            .results
            .items
            .iter()
            .find(|i| i.id.as_str() == result_id.as_str())
            .cloned()
        {
            return seed_module_add(state, &item);
        }
        return vec![Effect::None];
    }
    if action.id == "seed_config" || action.id == "configure" {
        state.route = Route::Search;
        if let Some(item) = state
            .search
            .results
            .items
            .iter()
            .find(|i| i.id.as_str() == result_id.as_str())
            .cloned()
        {
            return seed_module_config(state, &item);
        }
        return vec![Effect::None];
    }
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
        vec![Effect::None]
    } else {
        state.route = Route::Search;
        execute_action(state, result_id, action, false)
    }
}

pub(super) fn execute_action(
    state: &mut AppState,
    result_id: String,
    action: ActionDescriptorDto,
    confirmation: bool,
) -> Vec<Effect> {
    if state.actions.active_operation.is_some() {
        state.status.set(
            "action already running — Esc to cancel",
            StatusTone::Warning,
        );
        return vec![Effect::None];
    }
    let operation_id = next_operation_id(state);
    state.actions.active_operation = Some(operation_id.clone());
    state
        .status
        .set(format!("running {}", action.label), StatusTone::Progress);
    vec![Effect::ExecuteAction {
        operation_id,
        result_id,
        action_id: action.id,
        confirmation,
    }]
}

pub(super) fn begin_primary_or_confirm(
    state: &mut AppState,
    result_id: String,
    actions: Vec<ActionDescriptorDto>,
) -> Vec<Effect> {
    let primary_id = state
        .search
        .results
        .items
        .iter()
        .find(|i| i.id.as_str() == result_id)
        .map(|i| i.primary_action.id.as_str().to_string());
    let Some(primary_id) = primary_id else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    let Some(action) = actions.into_iter().find(|a| a.id == primary_id) else {
        state.status.set(
            format!("module contract violation: primary action `{primary_id}` missing"),
            StatusTone::Error,
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
        vec![Effect::None]
    } else {
        execute_action(state, result_id, action, false)
    }
}
