use crate::effect::Effect;
use crate::view_model::{AppState, Route, StatusTone};

use super::actions::clear_action_ui;

pub(super) const COMMANDS: &[(&str, &str)] = &[
    ("settings", "Open module settings"),
    ("help", "Keyboard help"),
    ("quit", "Quit Luma"),
];

pub(super) fn open_settings(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.route = Route::Settings;
    state.settings.selected = 0;
    state
        .status
        .set("settings · Space toggle · Esc back", StatusTone::Neutral);
    vec![Effect::GetSettings]
}

pub(super) fn open_commands(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.route = Route::Commands;
    state.overlay.commands_selected = 0;
    state
        .status
        .set("commands · Enter run · Esc back", StatusTone::Neutral);
    vec![Effect::None]
}

pub(super) fn run_command_selection(state: &mut AppState) -> Vec<Effect> {
    let idx = state.overlay.commands_selected.min(COMMANDS.len() - 1);
    match COMMANDS[idx].0 {
        "settings" => open_settings(state),
        "help" => {
            state.route = Route::Help;
            state.overlay.help_scroll = 0;
            state.status.set("help", StatusTone::Neutral);
            vec![Effect::None]
        }
        "quit" => {
            state.route = Route::QuitConfirm;
            state.status.set("Quit Luma?", StatusTone::Warning);
            vec![Effect::None]
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
