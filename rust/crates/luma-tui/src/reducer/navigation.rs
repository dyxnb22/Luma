use crate::effect::Effect;
use crate::view_model::{AppState, FocusZone, Route, StatusTone};
use luma_protocol::ActionDescriptorDto;

use super::actions::{clear_action_ui, execute_action, review_return_route};
use super::explicit_command_prompt;
use super::preview::preview_effect;
use super::search::{begin_search, cancel_active, schedule_search};
use super::wordbook;

pub(super) fn apply_hub_selection(state: &mut AppState) -> Vec<Effect> {
    let entries = state.hub_rows();
    if entries.is_empty() {
        state
            .status
            .set("waiting for modules…", StatusTone::Progress);
        state.schedule_hub_refresh();
        return vec![Effect::LoadHub];
    }
    let idx = state.hub.selected.min(entries.len() - 1);
    let (kind, id, title, query) = &entries[idx];
    if kind == "window" {
        return execute_action(
            state,
            id.clone(),
            ActionDescriptorDto {
                id: "focus".into(),
                label: format!("Focus {title}"),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            false,
        );
    }
    if kind == "window_status" || kind == "window_more" {
        state.search.prompt = query.clone();
        state.search.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.search.history_browse = None;
        state.search.browse_nav_stack.clear();
        if kind == "window_status" {
            if let Some(hub) = &state.hub.windows {
                if let Some(sub) = &hub.status_subtitle {
                    state.status.set(sub.clone(), StatusTone::Warning);
                }
            }
        }
        return schedule_search(state);
    }
    state.search.prompt = query.clone();
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.search.history_browse = None;
    state.search.browse_nav_stack.clear();
    schedule_search(state)
}

pub(super) fn drill_into_browse(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
) -> Vec<Effect> {
    // Navigation is an explicit module contract. Result IDs remain opaque and are never
    // reinterpreted as routing metadata by the reducer.
    let records_category = super::payload_str(item, "category").map(str::to_string);
    let trigger = super::payload_str(item, "browse_trigger").filter(|trigger| !trigger.is_empty());
    let Some(trigger) = trigger else {
        state
            .status
            .set("browse metadata missing", StatusTone::Error);
        return vec![Effect::None];
    };
    let path = if trigger == "rec" {
        records_category
    } else {
        super::payload_str(item, "path")
            .map(str::to_string)
            .or_else(|| item.subtitle.clone())
    };
    let query = match path {
        Some(p) if !p.is_empty() => format!("/{trigger} browse {p}"),
        _ => format!("/{trigger} browse"),
    };
    let previous = state.search.prompt.clone();
    if !previous.is_empty() && previous != query {
        state.search.browse_nav_stack.push(previous);
        if state.search.browse_nav_stack.len() > 64 {
            state.search.browse_nav_stack.remove(0);
        }
    }
    state.search.prompt = query;
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.search.history_browse = None;
    state.status.set("browsing…", StatusTone::Progress);
    begin_search(state)
}

pub(super) fn open_notes_issues(state: &mut AppState) -> Vec<Effect> {
    state.search.browse_nav_stack.clear();
    state.search.prompt = "/n issues".into();
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.search.history_browse = None;
    state.search.results.items.clear();
    state.search.results.selected_id = None;
    state.status.set("notes issues…", StatusTone::Progress);
    begin_search(state)
}

pub(super) fn seed_module_add(state: &mut AppState, item: &luma_domain::SearchItem) -> Vec<Effect> {
    let prompt = super::payload_str(item, "seed_prompt").unwrap_or_else(|| {
        if item.id.as_str().starts_with("ql:") {
            "ql add "
        } else if item.id.as_str().starts_with("snip:") {
            "snip add "
        } else {
            ""
        }
    });
    if prompt.is_empty() {
        return vec![Effect::None];
    }
    state.search.browse_nav_stack.clear();
    state.search.prompt = format!("/{}", prompt.trim_start_matches('/'));
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.search.history_browse = None;
    state.search.results.items.clear();
    state.search.results.selected_id = None;
    state.status.set(
        "type trigger and payload · Enter when ready",
        StatusTone::Neutral,
    );
    state.search.debounce_deadline = None;
    state.hub.refresh_deadline = None;
    vec![Effect::None]
}

pub(super) fn seed_record_edit(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
    action: &str,
) -> Vec<Effect> {
    let Some(id) = item.id.as_str().strip_prefix("rec:") else {
        state.status.set("invalid record id", StatusTone::Error);
        return vec![Effect::None];
    };
    state.search.browse_nav_stack.clear();
    state.search.prompt = match action {
        "rate" => format!("/rec rate {id} "),
        "note" => format!("/rec note {id} "),
        _ => return vec![Effect::None],
    };
    state.search.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.search.history_browse = None;
    state.search.results.items.clear();
    state.search.results.selected_id = None;
    state.preview.body = None;
    state.preview.result_id = None;
    state.status.set(
        "type value · Enter to save · Esc cancel",
        StatusTone::Neutral,
    );
    state.search.debounce_deadline = None;
    state.hub.refresh_deadline = None;
    vec![Effect::None]
}

pub(super) fn seed_module_config(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
) -> Vec<Effect> {
    if item.id.as_str() == "proj:not-configured" {
        state.status.set(
            "run in terminal: /proj add /path/to/project · or Enter on /proj browse",
            StatusTone::Warning,
        );
        return vec![Effect::None];
    }
    if let Some(cmd) = super::payload_str(item, "config_hint") {
        state
            .status
            .set(format!("run in terminal: {cmd}"), StatusTone::Warning);
        return vec![Effect::None];
    }
    let cmd = if item.id.as_str().starts_with("n:") || item.id.as_str().starts_with("notes:") {
        Some("luma config set --notes-root ~/Notes")
    } else if item.id.as_str().starts_with("proj:") {
        Some("luma config set --projects-root ~/dev")
    } else if item.id.as_str().starts_with("sec:") || item.kind == "secrets" {
        Some("luma secrets set <account>  (value from stdin)")
    } else {
        None
    };
    if let Some(cmd) = cmd {
        state
            .status
            .set(format!("run in terminal: {cmd}"), StatusTone::Warning);
        return vec![Effect::None];
    }
    if let Some(sub) = item.subtitle.as_deref() {
        state.status.set(sub, StatusTone::Warning);
        return vec![Effect::None];
    }
    state
        .status
        .set("configure via: luma config", StatusTone::Warning);
    vec![Effect::None]
}

pub(super) fn pick_window_digit(state: &mut AppState, digit: usize) -> Vec<Effect> {
    if digit == 0 || !state.should_intercept_window_digit() {
        return vec![Effect::None];
    }
    let targets = state.window_digit_targets();
    let idx = digit - 1;
    let Some((id, title)) = targets.get(idx).cloned() else {
        return vec![Effect::None];
    };
    execute_action(
        state,
        id,
        ActionDescriptorDto {
            id: "focus".into(),
            label: format!("Focus {title}"),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        },
        false,
    )
}

pub(super) fn cancel_msg(state: &mut AppState) -> Vec<Effect> {
    if let Some(operation_id) = state.actions.active_operation.clone() {
        state.status.set("cancelling action…", StatusTone::Progress);
        return vec![Effect::CancelOperation { operation_id }];
    }
    if state.route == Route::WordbookReview {
        return wordbook::exit_wordbook_review(state);
    }
    if matches!(state.route, Route::ConfirmAction | Route::ActionPicker) {
        clear_action_ui(state);
        if let Some(review) = state.wordbook.review.as_mut() {
            review.pending_grade = None;
        }
        state.route = review_return_route(state);
        state.status.set("Dismissed", StatusTone::Warning);
        return vec![Effect::None];
    }
    if state.route != Route::Search {
        state.route = Route::Search;
        if let Some(prompt) = state.overlay.restore_prompt.take() {
            state.search.prompt = prompt;
            state.search.prompt_cursor = state.prompt_char_len();
            state.focus = FocusZone::Prompt;
            state.status.set("Ready", StatusTone::Neutral);
            return vec![Effect::None];
        }
        if state.showing_hub() {
            state.status.set("Ready", StatusTone::Neutral);
            state.schedule_hub_refresh();
            return vec![Effect::LoadHub];
        }
        return vec![Effect::None];
    }
    if state.search.active_request.is_some() {
        let effects = cancel_active(state);
        state.status.set("cancelled", StatusTone::Warning);
        effects
    } else if let Some(prev) = state.search.browse_nav_stack.pop() {
        state.search.prompt = prev;
        state.search.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.search.history_browse = None;
        state.status.set("browsing…", StatusTone::Progress);
        begin_search(state)
    } else if let Some(parent) = browse_query_parent(&state.search.prompt) {
        state.search.prompt = parent;
        state.search.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.search.history_browse = None;
        state.status.set("browsing…", StatusTone::Progress);
        begin_search(state)
    } else if !state.search.prompt.is_empty() {
        clear_action_ui(state);
        state.search.browse_nav_stack.clear();
        state.clear_prompt();
        state.search.debounce_deadline = None;
        state.search.results.items.clear();
        state.search.results.selected_id = None;
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.pending_id = None;
        state.search.active_request = None;
        state.status.set("Ready", StatusTone::Neutral);
        state.schedule_hub_refresh();
        vec![Effect::LoadHub]
    } else {
        clear_action_ui(state);
        state.route = Route::QuitConfirm;
        state.status.set("Quit Luma?", StatusTone::Warning);
        vec![Effect::None]
    }
}

/// One directory up for slash-prefixed `/n|/note|/notes|/proj browse <path>`.
pub(super) fn browse_query_parent(prompt: &str) -> Option<String> {
    let trimmed = explicit_command_prompt(prompt)?;
    let (trigger, after_trigger) = if let Some(rest) = trimmed.strip_prefix("notes ") {
        ("notes", rest)
    } else if let Some(rest) = trimmed.strip_prefix("note ") {
        ("note", rest)
    } else if let Some(rest) = trimmed.strip_prefix("proj ") {
        ("proj", rest)
    } else if let Some(rest) = trimmed.strip_prefix("n ") {
        ("n", rest)
    } else {
        return None;
    };
    let after_browse = after_trigger
        .strip_prefix("browse")
        .or_else(|| after_trigger.strip_prefix("Browse"))
        .or_else(|| after_trigger.strip_prefix("ls"))
        .or_else(|| after_trigger.strip_prefix("LS"))?;
    let path = after_browse.trim();
    if path.is_empty() {
        return None;
    }
    let path = std::path::Path::new(path);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            Some(format!("/{trigger} browse {}", parent.display()))
        }
        _ => Some(format!("/{trigger} browse")),
    }
}

pub(super) fn select_next_msg(state: &mut AppState) -> Vec<Effect> {
    if state.route == Route::ActionPicker {
        if !state.actions.action_choices.is_empty() {
            state.actions.action_selected =
                (state.actions.action_selected + 1).min(state.actions.action_choices.len() - 1);
        }
        return vec![Effect::None];
    }
    if state.route == Route::Settings {
        if !state.settings.modules.is_empty() {
            state.settings.selected =
                (state.settings.selected + 1).min(state.settings.modules.len() - 1);
        }
        return vec![Effect::None];
    }
    if state.route == Route::Commands {
        state.overlay.commands_selected =
            (state.overlay.commands_selected + 1).min(super::overlays::COMMANDS.len() - 1);
        return vec![Effect::None];
    }
    if state.route == Route::Help {
        state.overlay.help_scroll = state.overlay.help_scroll.saturating_add(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.focus == FocusZone::Preview && state.preview_visible()
    {
        state.preview.scroll = state.preview.scroll.saturating_add(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search
        && state.search.prompt.is_empty()
        && state.search.results.items.is_empty()
    {
        let max = state.hub_rows().len().saturating_sub(1);
        state.focus = FocusZone::List;
        state.hub.selected = (state.hub.selected + 1).min(max);
        state.ensure_hub_selection_visible();
        return vec![Effect::None];
    }
    if state.route == Route::Search {
        state.focus = FocusZone::List;
        state.search.results.select_next();
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.scroll = 0;
        state.preview.pending_id = None;
        return preview_effect(state);
    }
    vec![Effect::None]
}

pub(super) fn select_prev_msg(state: &mut AppState) -> Vec<Effect> {
    if state.route == Route::ActionPicker {
        state.actions.action_selected = state.actions.action_selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Settings {
        state.settings.selected = state.settings.selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Commands {
        state.overlay.commands_selected = state.overlay.commands_selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Help {
        state.overlay.help_scroll = state.overlay.help_scroll.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.focus == FocusZone::Preview && state.preview_visible()
    {
        state.preview.scroll = state.preview.scroll.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search
        && state.search.prompt.is_empty()
        && state.search.results.items.is_empty()
    {
        state.focus = FocusZone::List;
        state.hub.selected = state.hub.selected.saturating_sub(1);
        state.ensure_hub_selection_visible();
        return vec![Effect::None];
    }
    if state.route == Route::Search {
        state.focus = FocusZone::List;
        state.search.results.select_prev();
        state.preview.body = None;
        state.preview.result_id = None;
        state.preview.pending_id = None;
        state.preview.scroll = 0;
        return preview_effect(state);
    }
    vec![Effect::None]
}
