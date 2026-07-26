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
use overlays::{open_commands, open_settings, run_command_selection, toggle_setting};
use preview::{preview_effect, sync_prompt_viewport};
use search::{begin_search, cancel_active, flush_pending_search_or_continue, schedule_search};

const PAGE_SIZE: usize = 5;

pub(crate) fn explicit_command_prompt(prompt: &str) -> Option<&str> {
    prompt.trim_start().strip_prefix('/').map(str::trim)
}

/// Parse workbench-owned settings commands. The input stays slash-prefixed, and persistence
/// goes through the Engine's versioned settings CAS.
fn settings_patch_from_prompt(
    prompt: &str,
    current_project_roots: &[String],
) -> Result<Option<serde_json::Value>, String> {
    let Some(command) = explicit_command_prompt(prompt) else {
        return Ok(None);
    };
    let Some(rest) = command.strip_prefix("settings") else {
        return Ok(None);
    };
    if !rest.is_empty() && !rest.chars().next().is_some_and(char::is_whitespace) {
        return Ok(None);
    }
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Ok(None);
    }
    let field_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let field = &rest[..field_end];
    // Preserve the path exactly apart from the command separator. Paths can
    // legitimately contain repeated spaces, which `split_whitespace` loses.
    let value = rest[field_end..].trim().to_string();
    let parse_u32 = |name: &str, min: u32, max: u32| -> Result<u32, String> {
        let parsed = value
            .parse::<u32>()
            .map_err(|_| format!("/settings {name} needs a number"))?;
        if !(min..=max).contains(&parsed) {
            return Err(format!("/settings {name} must be between {min} and {max}"));
        }
        Ok(parsed)
    };
    let patch = match field {
        "projects-root" => {
            if value.is_empty() {
                return Err("/settings projects-root needs a path".into());
            }
            let mut roots = current_project_roots.to_vec();
            if !roots.iter().any(|root| root == &value) {
                roots.push(value);
            }
            serde_json::json!({ "projects_roots": roots })
        }
        "import-project" => {
            if value.is_empty() {
                return Err("/settings import-project needs a path".into());
            }
            serde_json::json!({ "import_project": value })
        }
        "records-root" => {
            if value.is_empty() {
                return Err("/settings records-root needs PATH or none".into());
            }
            if matches!(value.as_str(), "none" | "off" | "-") {
                serde_json::json!({ "records_root": null })
            } else {
                serde_json::json!({ "records_root": value })
            }
        }
        "clipboard-retention-days" => {
            let days = parse_u32(field, 1, 3_650)?;
            serde_json::json!({ "clipboard_retention_days": days })
        }
        "secrets-idle-lock-secs" => {
            let seconds = parse_u32(field, 0, 2_592_000)?;
            serde_json::json!({ "secrets_idle_lock_secs": seconds })
        }
        "hub-windows-max" => {
            let rows = parse_u32(field, 5, 50)?;
            serde_json::json!({ "hub_windows_max": rows })
        }
        _ => {
            return Err(format!(
                "unknown /settings field: {field} (open /settings for available fields)"
            ))
        }
    };
    Ok(Some(patch))
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
        UiIntent::SeedAdd => navigation::seed_module_add(state, item),
        UiIntent::SeedConfig => navigation::seed_module_config(state, item),
        UiIntent::OpenPath => {
            state
                .status
                .set("open via action picker", StatusTone::Warning);
            vec![Effect::None]
        }
        UiIntent::OpenSurface => navigation::open_surface(state, item),
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
