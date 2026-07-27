use crate::effect::Effect;
use crate::view_model::{AppState, Route, StatusTone};

use super::actions::clear_action_ui;
use super::navigation::{scroll_page, ScrollDirection};

pub(super) fn open_settings(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.route = Route::Settings;
    state.overlay.commands_return_route = None;
    state.settings.selected = 0;
    state
        .status
        .set("settings · Space toggle · Esc back", StatusTone::Neutral);
    vec![Effect::GetSettings]
}

pub(super) fn open_commands(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Commands {
        state.overlay.commands_return_route = Some(state.route.clone());
    }
    state.route = Route::Commands;
    state.overlay.commands_selected = 0;
    state.overlay.commands_filter.clear();
    state
        .status
        .set("commands · Enter run · Esc back", StatusTone::Neutral);
    vec![Effect::None]
}

pub(super) fn run_command_selection(state: &mut AppState) -> Vec<Effect> {
    let rows = state.command_palette_rows();
    let Some(entry) = rows
        .get(
            state
                .overlay
                .commands_selected
                .min(rows.len().saturating_sub(1)),
        )
        .cloned()
    else {
        return vec![Effect::None];
    };
    match entry.id.as_str() {
        "settings" => open_settings(state),
        "scroll:up" | "scroll:down" => {
            state.route = state
                .overlay
                .commands_return_route
                .take()
                .unwrap_or(Route::Search);
            state.overlay.commands_filter.clear();
            scroll_page(
                state,
                if entry.id == "scroll:up" {
                    ScrollDirection::Up
                } else {
                    ScrollDirection::Down
                },
            )
        }
        "help" => {
            state.route = Route::Help;
            state.overlay.commands_return_route = None;
            state.overlay.help_scroll = 0;
            state.status.set("help", StatusTone::Neutral);
            vec![Effect::None]
        }
        "commands" => vec![Effect::None],
        "quit" => {
            clear_action_ui(state);
            state.route = Route::QuitConfirm;
            state.overlay.commands_return_route = None;
            state.status.set("Quit Luma?", StatusTone::Warning);
            vec![Effect::None]
        }
        _ if entry.query.is_some() => {
            let query = entry.query.expect("checked above");
            clear_action_ui(state);
            state.route = Route::Search;
            state.overlay.commands_return_route = None;
            state.overlay.restore_prompt = None;
            state.overlay.commands_filter.clear();
            state.search.prompt = query;
            state.search.prompt_cursor = state.prompt_char_len();
            state.focus = crate::view_model::FocusZone::Prompt;
            if entry.submit {
                super::search::begin_search(state)
            } else {
                state.search.debounce_deadline = None;
                state.search.results.items.clear();
                state.search.results.selected_id = None;
                state
                    .status
                    .set(format!("Complete {}", entry.label), StatusTone::Neutral);
                vec![Effect::None]
            }
        }
        _ => vec![Effect::None],
    }
}

pub(super) fn toggle_setting(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Settings || state.settings.modules.is_empty() {
        return vec![Effect::None];
    }
    let idx = state
        .settings
        .selected
        .min(state.settings.modules.len() - 1);
    let row = &state.settings.modules[idx];
    let module_id = row.id.clone();
    let enabled = !row.enabled;
    state.status.set(
        format!("{} → {}", module_id, if enabled { "on" } else { "off" }),
        StatusTone::Progress,
    );
    vec![Effect::UpdateSettings {
        module_id,
        enabled,
        expected_version: state.settings.version,
    }]
}
