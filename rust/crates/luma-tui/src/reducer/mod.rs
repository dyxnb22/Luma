use crate::effect::Effect;
use crate::msg::Msg;
use crate::view_model::{AppState, FocusZone, Route, StatusTone};
use luma_protocol::UiIntent;

mod actions;
mod dispatch;
mod engine;
mod navigation;
mod overlays;
mod preview;
mod search;
mod wordbook;

pub use dispatch::update;

use actions::{
    clear_action_ui, confirm_pending, dismiss_help_for_prompt_edit, recipe_shortcut,
    request_action_picker, request_primary_actions, submit_picker_selection,
};
use engine::apply_engine;
use navigation::{
    apply_hub_selection, cancel_msg, pick_window_digit, select_next_msg, select_prev_msg,
};
use overlays::{open_commands, open_settings, run_command_selection, toggle_setting, COMMANDS};
use preview::{preview_effect, sync_prompt_viewport};
use search::{begin_search, cancel_active, flush_pending_search_or_continue, schedule_search};

const PAGE_SIZE: usize = 5;

pub(crate) fn explicit_command_prompt(prompt: &str) -> Option<&str> {
    prompt.trim_start().strip_prefix('/').map(str::trim)
}

fn resolve_ui_intent(item: &luma_domain::SearchItem) -> Option<UiIntent> {
    item.ui_intent.as_deref().and_then(UiIntent::parse)
}

fn payload_str<'a>(item: &'a luma_domain::SearchItem, key: &str) -> Option<&'a str> {
    item.action_payload
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_str())
}

fn apply_ui_intent(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
    intent: UiIntent,
) -> Vec<Effect> {
    match intent {
        UiIntent::Browse => navigation::drill_into_browse(state, item),
        UiIntent::ListIssues => navigation::open_notes_issues(state),
        UiIntent::SeedAdd => navigation::seed_module_add(state, item),
        UiIntent::SeedConfig => navigation::seed_module_config(state, item),
        UiIntent::OpenPath => {
            state
                .status
                .set("open via action picker", StatusTone::Warning);
            vec![Effect::None]
        }
    }
}

fn records_query_active(prompt: &str) -> bool {
    let Some(command) = explicit_command_prompt(prompt) else {
        return false;
    };
    let lower = command.to_ascii_lowercase();
    matches!(
        lower.split_whitespace().next(),
        Some("rec") | Some("record")
    )
}

pub fn command_recipes_query_active(prompt: &str) -> bool {
    let Some(command) = explicit_command_prompt(prompt) else {
        return false;
    };
    matches!(
        command.split_whitespace().next(),
        Some("cmd") | Some("recipe") | Some("recipes")
    )
}

fn project_remove_name(prompt: &str) -> Option<&str> {
    let mut tokens = explicit_command_prompt(prompt)?.split_whitespace();
    let trigger = tokens.next()?.to_ascii_lowercase();
    if !matches!(trigger.as_str(), "p" | "proj" | "project") {
        return None;
    }
    if !tokens.next()?.eq_ignore_ascii_case("remove") {
        return None;
    }
    tokens.next().filter(|name| !name.is_empty())
}

fn next_operation_id(state: &mut AppState) -> String {
    state.actions.operation_generation = state.actions.operation_generation.saturating_add(1);
    format!("op-{}", state.actions.operation_generation)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;
