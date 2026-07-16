use crate::effect::Effect;
use crate::msg::Msg;
use crate::view_model::{
    ActionsIntent, AppState, AwaitingActions, FocusZone, PendingAction, Route, StatusTone,
};
use luma_protocol::{ActionDescriptorDto, Event, UiIntent};

const PAGE_SIZE: usize = 5;

fn resolve_ui_intent(item: &luma_domain::SearchItem) -> Option<UiIntent> {
    legacy_ui_intent_from_action(item)
}

fn legacy_ui_intent_from_action(item: &luma_domain::SearchItem) -> Option<UiIntent> {
    if item.kind == "directory" || item.primary_action.id.as_str() == "browse" {
        return Some(UiIntent::Browse);
    }
    match item.primary_action.id.as_str() {
        "list_issues" => Some(UiIntent::ListIssues),
        "seed_add" => Some(UiIntent::SeedAdd),
        "seed_config" | "configure" => Some(UiIntent::SeedConfig),
        _ => None,
    }
}

fn apply_ui_intent(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
    intent: UiIntent,
) -> Vec<Effect> {
    match intent {
        UiIntent::Browse => drill_into_browse(state, item),
        UiIntent::ListIssues => open_notes_issues(state),
        UiIntent::SeedAdd => seed_module_add(state, item),
        UiIntent::SeedConfig => seed_module_config(state, item),
        UiIntent::OpenPath => {
            state
                .status
                .set("open via action picker", StatusTone::Warning);
            vec![Effect::None]
        }
    }
}

/// Pure synchronous reducer. Must not perform I/O.
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    state.dirty = true;
    match msg {
        Msg::RecipeShortcut { action_id } => recipe_shortcut(state, &action_id),
        Msg::KeyChar(c) => {
            if matches!(
                state.route,
                Route::ConfirmAction
                    | Route::ActionPicker
                    | Route::Help
                    | Route::Settings
                    | Route::Commands
                    | Route::QuitConfirm
            ) {
                clear_action_ui(state);
                state.route = Route::Search;
            }
            state.focus = FocusZone::Prompt;
            state.history_browse = None;
            state.browse_nav_stack.clear();
            state.insert_prompt_char(c);
            sync_prompt_viewport(state);
            schedule_search(state)
        }
        Msg::Backspace => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.history_browse = None;
            state.browse_nav_stack.clear();
            state.backspace_prompt();
            schedule_search(state)
        }
        Msg::DeleteForward => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.history_browse = None;
            state.browse_nav_stack.clear();
            state.delete_forward_prompt();
            schedule_search(state)
        }
        Msg::CursorLeft => {
            if matches!(state.route, Route::Search | Route::Help) {
                state.focus = FocusZone::Prompt;
                state.clamp_prompt_cursor();
                state.prompt_cursor = state.prompt_cursor.saturating_sub(1);
            }
            vec![Effect::None]
        }
        Msg::CursorRight => {
            if matches!(state.route, Route::Search | Route::Help) {
                state.focus = FocusZone::Prompt;
                state.clamp_prompt_cursor();
                if state.prompt_cursor < state.prompt_char_len() {
                    state.prompt_cursor += 1;
                }
            }
            vec![Effect::None]
        }
        Msg::CursorHome => {
            if matches!(state.route, Route::Search | Route::Help) {
                state.focus = FocusZone::Prompt;
                state.prompt_cursor = 0;
            }
            vec![Effect::None]
        }
        Msg::CursorEnd => {
            if matches!(state.route, Route::Search | Route::Help) {
                state.focus = FocusZone::Prompt;
                state.prompt_cursor = state.prompt_char_len();
            }
            vec![Effect::None]
        }
        Msg::ClearToStart => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.history_browse = None;
            state.browse_nav_stack.clear();
            state.clear_prompt_to_start();
            schedule_search(state)
        }
        Msg::DeleteWordBack => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.history_browse = None;
            state.browse_nav_stack.clear();
            state.delete_prompt_word_back();
            schedule_search(state)
        }
        Msg::Submit => match state.route {
            Route::ConfirmAction => confirm_pending(state),
            Route::ActionPicker => submit_picker_selection(state),
            Route::QuitConfirm => {
                state.should_quit = true;
                cancel_active(state)
            }
            Route::Search | Route::Help => {
                if state.prompt.trim().is_empty()
                    && matches!(state.route, Route::Search)
                    && state.results.items.is_empty()
                {
                    return apply_hub_selection(state);
                }
                // Meta commands are local navigation. They must win over a pending
                // search debounce so one Enter opens the requested surface.
                if let Some(queue) = wordbook_review_queue_from_prompt(state.prompt.trim()) {
                    return begin_wordbook_review(state, queue);
                }
                if state.prompt.trim() == ":settings" {
                    state.overlay_restore_prompt = Some(state.prompt.clone());
                    state.clear_prompt();
                    state.search_debounce_deadline = None;
                    return open_settings(state);
                }
                if state.prompt.trim() == ":help" || state.prompt.trim() == "?" {
                    state.overlay_restore_prompt = Some(state.prompt.clone());
                    state.clear_prompt();
                    state.search_debounce_deadline = None;
                    state.route = Route::Help;
                    state.help_scroll = 0;
                    state.status.set("help", StatusTone::Neutral);
                    return vec![Effect::None];
                }
                if state.prompt.trim() == ":commands" {
                    state.overlay_restore_prompt = Some(state.prompt.clone());
                    state.clear_prompt();
                    state.search_debounce_deadline = None;
                    return open_commands(state);
                }
                if let Some(effects) = flush_pending_search_or_continue(state) {
                    return effects;
                }
                if state.command_recipes_selected() && state.focus != FocusZone::Prompt {
                    state.preview_pinned = true;
                    return preview_effect(state);
                }
                request_primary_actions(state)
            }
            Route::Settings => toggle_setting(state),
            Route::Commands => run_command_selection(state),
            Route::WordbookReview => wordbook_reveal(state),
        },
        Msg::OpenActions => {
            if let Some(effects) = flush_pending_search_or_continue(state) {
                return effects;
            }
            request_action_picker(state)
        }
        Msg::OpenSettings => open_settings(state),
        Msg::OpenCommands => open_commands(state),
        Msg::ToggleSetting => toggle_setting(state),
        Msg::FocusNext => {
            if state.route == Route::Search {
                state.cycle_focus();
            }
            vec![Effect::None]
        }
        Msg::HistoryOlder => {
            if state.route == Route::Search {
                state.focus = FocusZone::Prompt;
                state.history_older();
                schedule_search(state)
            } else {
                vec![Effect::None]
            }
        }
        Msg::HistoryNewer => {
            if state.route == Route::Search {
                state.focus = FocusZone::Prompt;
                state.history_newer();
                schedule_search(state)
            } else {
                vec![Effect::None]
            }
        }
        Msg::SelectNext => select_next_msg(state),
        Msg::SelectPrev => select_prev_msg(state),
        Msg::SelectPageUp => {
            if state.route == Route::ActionPicker {
                state.action_selected = state.action_selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Help {
                state.help_scroll = state.help_scroll.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Settings {
                state.settings_selected = state.settings_selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Commands {
                state.commands_selected = state.commands_selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if matches!(state.route, Route::Search) {
                if state.focus == FocusZone::Preview && state.preview_visible() {
                    state.preview_scroll = state.preview_scroll.saturating_sub(PAGE_SIZE);
                    return vec![Effect::None];
                }
                if state.prompt.is_empty() && state.results.items.is_empty() {
                    state.hub_selected = state.hub_selected.saturating_sub(PAGE_SIZE);
                    state.ensure_hub_selection_visible();
                } else {
                    state.focus = FocusZone::List;
                    state.results.select_offset(-(PAGE_SIZE as isize));
                    state.preview_body = None;
                    state.preview_result_id = None;
                    state.pending_preview_id = None;
                    state.preview_scroll = 0;
                    return preview_effect(state);
                }
            }
            vec![Effect::None]
        }
        Msg::SelectPageDown => {
            if state.route == Route::ActionPicker {
                if !state.action_choices.is_empty() {
                    state.action_selected =
                        (state.action_selected + PAGE_SIZE).min(state.action_choices.len() - 1);
                }
                return vec![Effect::None];
            }
            if state.route == Route::Help {
                state.help_scroll = state.help_scroll.saturating_add(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Settings {
                if !state.settings_modules.is_empty() {
                    state.settings_selected =
                        (state.settings_selected + PAGE_SIZE).min(state.settings_modules.len() - 1);
                }
                return vec![Effect::None];
            }
            if state.route == Route::Commands {
                state.commands_selected =
                    (state.commands_selected + PAGE_SIZE).min(COMMANDS.len() - 1);
                return vec![Effect::None];
            }
            if matches!(state.route, Route::Search) {
                if state.focus == FocusZone::Preview && state.preview_visible() {
                    state.preview_scroll = state.preview_scroll.saturating_add(PAGE_SIZE);
                    return vec![Effect::None];
                }
                if state.prompt.is_empty() && state.results.items.is_empty() {
                    let max = state.hub_rows().len().saturating_sub(1);
                    state.hub_selected = (state.hub_selected + PAGE_SIZE).min(max);
                    state.ensure_hub_selection_visible();
                } else {
                    state.focus = FocusZone::List;
                    state.results.select_offset(PAGE_SIZE as isize);
                    state.preview_body = None;
                    state.preview_result_id = None;
                    state.pending_preview_id = None;
                    state.preview_scroll = 0;
                    return preview_effect(state);
                }
            }
            vec![Effect::None]
        }
        Msg::PickActionDigit(digit) => {
            if state.route != Route::ActionPicker || digit == 0 {
                return vec![Effect::None];
            }
            let idx = digit - 1;
            if idx >= state.action_choices.len() {
                return vec![Effect::None];
            }
            state.action_selected = idx;
            submit_picker_selection(state)
        }
        Msg::PickWindowDigit(digit) => pick_window_digit(state, digit),
        Msg::WordbookReveal => wordbook_reveal(state),
        Msg::WordbookGrade { action_id } => wordbook_grade(state, action_id),
        Msg::WordbookReviewExit => exit_wordbook_review(state),
        Msg::OpenHelp => {
            state.route = Route::Help;
            state.help_scroll = 0;
            state.status.set("help", StatusTone::Neutral);
            vec![Effect::None]
        }
        Msg::Quit => {
            if state.route == Route::QuitConfirm {
                state.should_quit = true;
                cancel_active(state)
            } else {
                clear_action_ui(state);
                state.route = Route::QuitConfirm;
                state.status.set("Quit Luma?", StatusTone::Warning);
                vec![Effect::None]
            }
        }
        Msg::Cancel => cancel_msg(state),
        Msg::FlushSearch => {
            state.search_debounce_deadline = None;
            begin_search(state)
        }
        Msg::Resize { width, height } => {
            state.term_width = width;
            state.term_height = height;
            state.sync_results_viewport();
            sync_prompt_viewport(state);
            if !state.preview_visible() && state.focus == FocusZone::Preview {
                state.focus = FocusZone::List;
            }
            vec![Effect::None]
        }
        Msg::Redraw | Msg::Tick => vec![Effect::None],
        Msg::RefreshHub => {
            // Soft refresh must not flash the whole UI every interval.
            state.dirty = false;
            if !state.showing_hub() {
                state.hub_refresh_deadline = None;
                return vec![Effect::None];
            }
            state.schedule_hub_refresh();
            vec![Effect::LoadHub]
        }
        Msg::BroadcastLagged => {
            state
                .status
                .set("Resyncing…", crate::view_model::StatusTone::Warning);
            if state.search_debounce_deadline.is_some() {
                state.search_debounce_deadline = None;
                return begin_search(state);
            }
            if state.active_request.is_some() || !state.prompt.trim().is_empty() {
                return begin_search(state);
            }
            vec![Effect::GetSnapshot]
        }
        Msg::TogglePreview => {
            if matches!(state.route, Route::Search) {
                state.preview_pinned = !state.preview_pinned;
                state.sync_results_viewport();
                return preview_effect(state);
            }
            vec![Effect::None]
        }
        Msg::FocusGained => {
            if state.showing_hub() {
                state.schedule_hub_refresh();
                vec![Effect::LoadHub]
            } else {
                vec![Effect::None]
            }
        }
        Msg::Engine(event) => apply_engine(state, event),
    }
}

fn sync_prompt_viewport(state: &mut AppState) {
    let inner_w = state.term_width.saturating_sub(2) as usize;
    state.ensure_prompt_visible(inner_w.max(20));
}

fn preview_effect(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.results.selected_id.clone() else {
        state.pending_preview_id = None;
        return vec![Effect::None];
    };
    // Already have body for this selection.
    if state.preview_result_id.as_deref() == Some(result_id.as_str())
        && state.preview_body.is_some()
    {
        return vec![Effect::None];
    }
    // In-flight request for this selection — don't spam.
    if state.pending_preview_id.is_some()
        && state.preview_result_id.as_deref() == Some(result_id.as_str())
    {
        return vec![Effect::None];
    }
    state.preview_generation = state.preview_generation.saturating_add(1);
    let preview_id = state.preview_generation;
    state.pending_preview_id = Some(preview_id);
    state.preview_result_id = Some(result_id.clone());
    state.preview_body = None;
    vec![Effect::LoadPreview {
        result_id,
        preview_id,
    }]
}

fn select_next_msg(state: &mut AppState) -> Vec<Effect> {
    if state.route == Route::ActionPicker {
        if !state.action_choices.is_empty() {
            state.action_selected = (state.action_selected + 1).min(state.action_choices.len() - 1);
        }
        return vec![Effect::None];
    }
    if state.route == Route::Settings {
        if !state.settings_modules.is_empty() {
            state.settings_selected =
                (state.settings_selected + 1).min(state.settings_modules.len() - 1);
        }
        return vec![Effect::None];
    }
    if state.route == Route::Commands {
        state.commands_selected = (state.commands_selected + 1).min(COMMANDS.len() - 1);
        return vec![Effect::None];
    }
    if state.route == Route::Help {
        state.help_scroll = state.help_scroll.saturating_add(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.focus == FocusZone::Preview && state.preview_visible()
    {
        state.preview_scroll = state.preview_scroll.saturating_add(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.prompt.is_empty() && state.results.items.is_empty() {
        let max = state.hub_rows().len().saturating_sub(1);
        state.focus = FocusZone::List;
        state.hub_selected = (state.hub_selected + 1).min(max);
        state.ensure_hub_selection_visible();
        return vec![Effect::None];
    }
    if matches!(state.route, Route::Search) {
        state.focus = FocusZone::List;
        state.results.select_next();
        state.preview_body = None;
        state.preview_result_id = None;
        state.preview_scroll = 0;
        state.pending_preview_id = None;
        return preview_effect(state);
    }
    vec![Effect::None]
}

fn select_prev_msg(state: &mut AppState) -> Vec<Effect> {
    if state.route == Route::ActionPicker {
        state.action_selected = state.action_selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Settings {
        state.settings_selected = state.settings_selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Commands {
        state.commands_selected = state.commands_selected.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Help {
        state.help_scroll = state.help_scroll.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.focus == FocusZone::Preview && state.preview_visible()
    {
        state.preview_scroll = state.preview_scroll.saturating_sub(1);
        return vec![Effect::None];
    }
    if state.route == Route::Search && state.prompt.is_empty() && state.results.items.is_empty() {
        state.focus = FocusZone::List;
        state.hub_selected = state.hub_selected.saturating_sub(1);
        state.ensure_hub_selection_visible();
        return vec![Effect::None];
    }
    if matches!(state.route, Route::Search) {
        state.focus = FocusZone::List;
        state.results.select_prev();
        state.preview_body = None;
        state.preview_result_id = None;
        state.pending_preview_id = None;
        state.preview_scroll = 0;
        return preview_effect(state);
    }
    vec![Effect::None]
}

const COMMANDS: &[(&str, &str)] = &[
    ("settings", "Open module settings"),
    ("help", "Keyboard help"),
    ("quit", "Quit Luma"),
];

fn open_settings(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.route = Route::Settings;
    state.settings_selected = 0;
    state
        .status
        .set("settings · Space toggle · Esc back", StatusTone::Neutral);
    vec![Effect::GetSettings]
}

fn open_commands(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.route = Route::Commands;
    state.commands_selected = 0;
    state
        .status
        .set("commands · Enter run · Esc back", StatusTone::Neutral);
    vec![Effect::None]
}

fn run_command_selection(state: &mut AppState) -> Vec<Effect> {
    let idx = state.commands_selected.min(COMMANDS.len() - 1);
    match COMMANDS[idx].0 {
        "settings" => open_settings(state),
        "help" => {
            state.route = Route::Help;
            state.help_scroll = 0;
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

fn toggle_setting(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Settings || state.settings_modules.is_empty() {
        return vec![Effect::None];
    }
    let idx = state
        .settings_selected
        .min(state.settings_modules.len() - 1);
    let row = &state.settings_modules[idx];
    let module_id = row.id.clone();
    let enabled = !row.enabled;
    state.status.set(
        format!("{} → {}", module_id, if enabled { "on" } else { "off" }),
        StatusTone::Progress,
    );
    vec![Effect::UpdateSettings {
        module_id,
        enabled,
        expected_version: state.settings_version,
    }]
}

fn apply_hub_selection(state: &mut AppState) -> Vec<Effect> {
    let entries = state.hub_rows();
    if entries.is_empty() {
        state
            .status
            .set("waiting for modules…", StatusTone::Progress);
        state.schedule_hub_refresh();
        return vec![Effect::LoadHub];
    }
    let idx = state.hub_selected.min(entries.len() - 1);
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
        state.prompt = query.clone();
        state.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.history_browse = None;
        state.browse_nav_stack.clear();
        if kind == "window_status" {
            if let Some(hub) = &state.hub_windows {
                if let Some(sub) = &hub.status_subtitle {
                    state.status.set(sub.clone(), StatusTone::Warning);
                }
            }
        }
        return schedule_search(state);
    }
    state.prompt = query.clone();
    state.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.history_browse = None;
    state.browse_nav_stack.clear();
    schedule_search(state)
}

fn clear_action_ui(state: &mut AppState) {
    state.awaiting_actions = None;
    state.pending_action = None;
    state.action_choices.clear();
    state.action_result_id = None;
    state.action_selected = 0;
}

fn recipe_shortcut(state: &mut AppState, action_id: &str) -> Vec<Effect> {
    let Some(item) = state.selected_search_item().cloned() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    if item.module_id.as_str() != "luma.command_recipes" {
        return vec![Effect::None];
    }
    let result_id = item.id.as_str().to_string();
    state.awaiting_actions = Some(AwaitingActions {
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

fn request_primary_actions(state: &mut AppState) -> Vec<Effect> {
    let Some(item) = state
        .results
        .selected_id
        .as_ref()
        .and_then(|id| {
            state
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
    if let Some(queue) = wordbook_review_queue_from_item(&item) {
        return begin_wordbook_review(state, queue);
    }
    if let Some(intent) = resolve_ui_intent(&item) {
        return apply_ui_intent(state, &item, intent);
    }
    let result_id = item.id.as_str().to_string();
    state.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Primary,
        result_id: result_id.clone(),
    });
    state.status.set("resolving actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

fn drill_into_browse(state: &mut AppState, item: &luma_domain::SearchItem) -> Vec<Effect> {
    let is_records = item.module_id.as_str().contains("records");
    let path = if is_records {
        item.action_payload
            .as_ref()
            .and_then(|p| p.get("category"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                item.id
                    .as_str()
                    .strip_prefix("rec:cat:")
                    .map(str::to_string)
            })
    } else {
        item.subtitle
            .clone()
            .or_else(|| {
                item.id
                    .as_str()
                    .strip_prefix("browse:proj:")
                    .or_else(|| item.id.as_str().strip_prefix("browse:n:"))
                    .map(str::to_string)
            })
            .or_else(|| item.id.as_str().strip_prefix("proj:").map(str::to_string))
    };
    let trigger = if is_records {
        "rec"
    } else if item.module_id.as_str().contains("notes") {
        "n"
    } else {
        "proj"
    };
    let query = match path {
        Some(p) if !p.is_empty() => format!("{trigger} browse {p}"),
        _ => format!("{trigger} browse"),
    };
    let previous = state.prompt.clone();
    if !previous.is_empty() && previous != query {
        state.browse_nav_stack.push(previous);
        if state.browse_nav_stack.len() > 64 {
            state.browse_nav_stack.remove(0);
        }
    }
    state.prompt = query;
    state.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.history_browse = None;
    state.status.set("browsing…", StatusTone::Progress);
    begin_search(state)
}

fn open_notes_issues(state: &mut AppState) -> Vec<Effect> {
    state.browse_nav_stack.clear();
    state.prompt = "n issues".into();
    state.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.history_browse = None;
    state.results.items.clear();
    state.results.selected_id = None;
    state.status.set("notes issues…", StatusTone::Progress);
    begin_search(state)
}

fn seed_module_add(state: &mut AppState, item: &luma_domain::SearchItem) -> Vec<Effect> {
    let prompt = if item.module_id.as_str().contains("quicklinks") {
        "ql add "
    } else if item.module_id.as_str().contains("snippets") {
        "snip add "
    } else {
        return vec![Effect::None];
    };
    state.browse_nav_stack.clear();
    state.prompt = prompt.into();
    state.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.history_browse = None;
    state.results.items.clear();
    state.results.selected_id = None;
    state.status.set(
        "type trigger and payload · Enter when ready",
        StatusTone::Neutral,
    );
    // Keep debounce quiet so the user can finish typing the add line.
    state.search_debounce_deadline = None;
    state.hub_refresh_deadline = None;
    vec![Effect::None]
}

fn seed_record_edit(
    state: &mut AppState,
    item: &luma_domain::SearchItem,
    action: &str,
) -> Vec<Effect> {
    let Some(id) = item.id.as_str().strip_prefix("rec:") else {
        state.status.set("invalid record id", StatusTone::Error);
        return vec![Effect::None];
    };
    state.browse_nav_stack.clear();
    state.prompt = match action {
        "rate" => format!("rec rate {id} "),
        "note" => format!("rec note {id} "),
        _ => return vec![Effect::None],
    };
    state.prompt_cursor = state.prompt_char_len();
    state.focus = FocusZone::Prompt;
    state.history_browse = None;
    state.results.items.clear();
    state.results.selected_id = None;
    state.preview_body = None;
    state.preview_result_id = None;
    state.status.set(
        "type value · Enter to save · Esc cancel",
        StatusTone::Neutral,
    );
    state.search_debounce_deadline = None;
    state.hub_refresh_deadline = None;
    vec![Effect::None]
}

fn seed_module_config(state: &mut AppState, item: &luma_domain::SearchItem) -> Vec<Effect> {
    if item.id.as_str() == "proj:not-configured" {
        state.status.set(
            "run in terminal: proj add /path/to/project · or Enter on proj browse",
            StatusTone::Warning,
        );
        return vec![Effect::None];
    }
    let cmd = if item.module_id.as_str().contains("notes") {
        "luma config set --notes-root ~/Notes"
    } else if item.module_id.as_str().contains("projects") {
        "luma config set --projects-root ~/dev"
    } else if item.module_id.as_str().contains("secrets") {
        "luma secrets set <account>  (value from stdin)"
    } else if let Some(sub) = item.subtitle.as_deref() {
        // Fall back to subtitle when it already carries a CLI hint.
        state.status.set(sub, StatusTone::Warning);
        return vec![Effect::None];
    } else {
        state
            .status
            .set("configure via: luma config", StatusTone::Warning);
        return vec![Effect::None];
    };
    state
        .status
        .set(format!("run in terminal: {cmd}"), StatusTone::Warning);
    vec![Effect::None]
}

fn request_action_picker(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::Search {
        return vec![Effect::None];
    }
    let Some(result_id) = state.results.selected_id.clone() else {
        state.status.set("no result selected", StatusTone::Warning);
        return vec![Effect::None];
    };
    state.awaiting_actions = Some(AwaitingActions {
        intent: ActionsIntent::Picker,
        result_id: result_id.clone(),
    });
    state.status.set("loading actions…", StatusTone::Progress);
    vec![Effect::ListActions { result_id }]
}

fn review_return_route(state: &AppState) -> Route {
    if state.wordbook_review.is_some() {
        Route::WordbookReview
    } else {
        Route::Search
    }
}

fn confirm_pending(state: &mut AppState) -> Vec<Effect> {
    let Some(pending) = state.pending_action.take() else {
        state.route = review_return_route(state);
        return vec![Effect::None];
    };
    state.route = review_return_route(state);
    execute_action(state, pending.result_id, pending.action, true)
}

fn submit_picker_selection(state: &mut AppState) -> Vec<Effect> {
    let Some(result_id) = state.action_result_id.take() else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    let Some(action) = state.action_choices.get(state.action_selected).cloned() else {
        state.route = Route::Search;
        clear_action_ui(state);
        return vec![Effect::None];
    };
    state.action_choices.clear();
    state.action_selected = 0;
    if matches!(action.id.as_str(), "rate" | "note") {
        if let Some(item) = state
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
        state.pending_action = Some(PendingAction {
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

fn execute_action(
    state: &mut AppState,
    result_id: String,
    action: ActionDescriptorDto,
    confirmation: bool,
) -> Vec<Effect> {
    if state.active_operation.is_some() {
        state.status.set(
            "action already running — Esc to cancel",
            StatusTone::Warning,
        );
        return vec![Effect::None];
    }
    state.search_generation = state.search_generation.saturating_add(1);
    let operation_id = format!("op-{}", state.search_generation);
    state.active_operation = Some(operation_id.clone());
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

fn begin_primary_or_confirm(
    state: &mut AppState,
    result_id: String,
    actions: Vec<ActionDescriptorDto>,
) -> Vec<Effect> {
    let primary_id = state
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
        state.pending_action = Some(PendingAction {
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

fn wordbook_review_queue_from_prompt(prompt: &str) -> Option<String> {
    let lower = prompt.to_ascii_lowercase();
    if lower == "wb review" || lower == "wb review due" {
        return Some("due".into());
    }
    if lower == "wb review new" {
        return Some("new".into());
    }
    if lower == "wb review wrong" {
        return Some("wrong".into());
    }
    None
}

fn wordbook_review_queue_from_item(item: &luma_domain::SearchItem) -> Option<String> {
    if item.primary_action.id.as_str() == "start_review" {
        return item
            .action_payload
            .as_ref()
            .and_then(|p| p.get("queue"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                item.id
                    .as_str()
                    .strip_prefix("wb:review:")
                    .map(str::to_string)
            });
    }
    item.id
        .as_str()
        .strip_prefix("wb:review:")
        .map(str::to_string)
}

fn begin_wordbook_review(state: &mut AppState, queue: String) -> Vec<Effect> {
    state.overlay_restore_prompt = Some(state.prompt.clone());
    state.clear_prompt();
    state.search_debounce_deadline = None;
    state.route = Route::WordbookReview;
    state.wordbook_review = None;
    state
        .status
        .set(format!("loading review ({queue})…"), StatusTone::Progress);
    vec![Effect::LoadWordbookReview { queue }]
}

fn pick_window_digit(state: &mut AppState, digit: usize) -> Vec<Effect> {
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
        luma_protocol::ActionDescriptorDto {
            id: "focus".into(),
            label: format!("Focus {title}"),
            risk: luma_domain::ActionRisk::Safe,
            confirmation: false,
        },
        false,
    )
}

fn wordbook_reveal(state: &mut AppState) -> Vec<Effect> {
    if state.route != Route::WordbookReview {
        return vec![Effect::None];
    }
    if let Some(review) = state.wordbook_review.as_mut() {
        if !review.finished {
            review.revealed = true;
        }
    }
    vec![Effect::None]
}

fn wordbook_grade(state: &mut AppState, action_id: String) -> Vec<Effect> {
    if state.route != Route::WordbookReview {
        return vec![Effect::None];
    }
    let Some(review) = state.wordbook_review.as_ref() else {
        return vec![Effect::None];
    };
    if review.finished || state.active_operation.is_some() {
        return vec![Effect::None];
    }
    let Some(word_id) = review.words.get(review.index).map(|w| w.id) else {
        return vec![Effect::None];
    };
    let revealed = review.revealed;
    if action_id == "skip" {
        if let Some(review) = state.wordbook_review.as_mut() {
            review.stats.session_skipped += 1;
            review.revealed = false;
            review.index += 1;
            if review.index >= review.words.len() {
                review.finished = true;
            }
        }
        if state.wordbook_review.as_ref().is_some_and(|r| r.finished) {
            state
                .status
                .set("review done · skipped", StatusTone::Success);
        }
        return vec![Effect::None];
    }
    if !revealed {
        return vec![Effect::None];
    }
    let result_id = format!("wb:{word_id}");
    let mastered = action_id == "mastered";
    let action = luma_protocol::ActionDescriptorDto {
        id: action_id.clone(),
        label: action_id.clone(),
        risk: if mastered {
            luma_domain::ActionRisk::Confirm
        } else {
            luma_domain::ActionRisk::Safe
        },
        confirmation: mastered,
    };
    if let Some(review) = state.wordbook_review.as_mut() {
        review.pending_grade = Some(action_id.clone());
    }
    if mastered {
        state.pending_action = Some(PendingAction { result_id, action });
        state.route = Route::ConfirmAction;
        state
            .status
            .set("confirm mastered? Enter=yes Esc=no", StatusTone::Warning);
        return vec![Effect::None];
    }
    execute_action(state, result_id, action, false)
}

fn exit_wordbook_review(state: &mut AppState) -> Vec<Effect> {
    state.wordbook_review = None;
    state.route = Route::Search;
    if let Some(prompt) = state.overlay_restore_prompt.take() {
        state.prompt = prompt;
        state.prompt_cursor = state.prompt_char_len();
    }
    state.focus = FocusZone::Prompt;
    state.status.set("review ended", StatusTone::Neutral);
    vec![Effect::None]
}

fn cancel_msg(state: &mut AppState) -> Vec<Effect> {
    if let Some(operation_id) = state.active_operation.clone() {
        state.status.set("cancelling action…", StatusTone::Progress);
        return vec![Effect::CancelOperation { operation_id }];
    }
    if state.route == Route::WordbookReview {
        return exit_wordbook_review(state);
    }
    if matches!(state.route, Route::ConfirmAction | Route::ActionPicker) {
        clear_action_ui(state);
        if let Some(review) = state.wordbook_review.as_mut() {
            review.pending_grade = None;
        }
        state.route = review_return_route(state);
        state.status.set("cancelled", StatusTone::Warning);
        return vec![Effect::None];
    }
    if state.route != Route::Search {
        state.route = Route::Search;
        if let Some(prompt) = state.overlay_restore_prompt.take() {
            state.prompt = prompt;
            state.prompt_cursor = state.prompt_char_len();
            state.focus = FocusZone::Prompt;
            state.status.set("Ready", StatusTone::Neutral);
            return vec![Effect::None];
        }
        if state.showing_hub() {
            state.status.set("Ready", StatusTone::Success);
            state.schedule_hub_refresh();
            return vec![Effect::LoadHub];
        }
        return vec![Effect::None];
    }
    if state.active_request.is_some() {
        let effects = cancel_active(state);
        state.status.set("cancelled", StatusTone::Warning);
        effects
    } else if let Some(prev) = state.browse_nav_stack.pop() {
        state.prompt = prev;
        state.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.history_browse = None;
        state.status.set("browsing…", StatusTone::Progress);
        begin_search(state)
    } else if let Some(parent) = browse_query_parent(&state.prompt) {
        state.prompt = parent;
        state.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::Prompt;
        state.history_browse = None;
        state.status.set("browsing…", StatusTone::Progress);
        begin_search(state)
    } else if !state.prompt.is_empty() {
        state.browse_nav_stack.clear();
        state.clear_prompt();
        state.search_debounce_deadline = None;
        state.results.items.clear();
        state.results.selected_id = None;
        state.active_request = None;
        state.status.set("Ready", StatusTone::Success);
        state.schedule_hub_refresh();
        vec![Effect::LoadHub]
    } else {
        // Same path as Ctrl-C — confirm before leaving the workbench.
        clear_action_ui(state);
        state.route = Route::QuitConfirm;
        state.status.set("Quit Luma?", StatusTone::Warning);
        vec![Effect::None]
    }
}

/// One directory up for `n|note|notes|proj browse <path>`; `None` at browse root / non-browse.
fn browse_query_parent(prompt: &str) -> Option<String> {
    let trimmed = prompt.trim();
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
            Some(format!("{trigger} browse {}", parent.display()))
        }
        _ => Some(format!("{trigger} browse")),
    }
}

fn apply_engine(state: &mut AppState, event: Event) -> Vec<Effect> {
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
        if state.active_operation.as_deref() == Some(operation_id.as_str()) {
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
        let Some(pending) = state.awaiting_actions.take() else {
            state.status.set(
                format!("{result_id}: {} actions", actions.len()),
                StatusTone::Neutral,
            );
            return vec![Effect::None];
        };
        if pending.result_id != result_id {
            // Stale / mismatched response for a different result — keep waiting.
            state.awaiting_actions = Some(pending);
            return vec![Effect::None];
        }
        match pending.intent {
            ActionsIntent::Primary => {
                return begin_primary_or_confirm(state, result_id, actions);
            }
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
                    state.pending_action = Some(PendingAction {
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
                state.action_result_id = Some(result_id);
                state.action_choices = actions;
                state.action_selected = 0;
                state.route = Route::ActionPicker;
                state
                    .status
                    .set("pick action · Enter run · Esc back", StatusTone::Neutral);
                return vec![Effect::None];
            }
        }
    }
    let project_remove_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && project_remove_name(&state.prompt).is_some());
    let records_mutation_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && records_query_active(&state.prompt));
    let cmd_favorite_success = matches!(&event, Event::ActionFinished { operation_id, outcome }
        if state.active_operation.as_deref() == Some(operation_id.as_str())
            && matches!(
                outcome,
                luma_protocol::ActionOutcomeDto::Success {
                    message: Some(message),
                    ..
                } if message == "favorited" || message == "unfavorited"
            )
            && command_recipes_query_active(&state.prompt));
    let refresh_review_stats = matches!(&event, Event::ActionFinished { outcome, .. }
        if matches!(outcome, luma_protocol::ActionOutcomeDto::Success { .. })
            && matches!(state.route, Route::WordbookReview)
            && state
                .wordbook_review
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
        if state.results.selected_id.is_some() {
            effects.extend(preview_effect(state));
        }
        return effects;
    }
    if settings_changed {
        // Disabled modules may still have cached rows until we re-query.
        state.results.items.clear();
        state.results.selected_id = None;
        state.preview_body = None;
        state.preview_result_id = None;
        state.pending_preview_id = None;
        clear_action_ui(state);
        let mut effects = vec![Effect::LoadHub];
        state.schedule_hub_refresh();
        if !(state.prompt.is_empty()
            || (state.active_operation.is_some() && project_remove_name(&state.prompt).is_some()))
        {
            effects.extend(begin_search(state));
        }
        return effects;
    }
    if project_remove_success {
        if let Some(name) = project_remove_name(&state.prompt) {
            state.results.items.clear();
            state.results.selected_id = None;
            state.preview_body = None;
            state.preview_result_id = None;
            state.pending_preview_id = None;
            state.status.set(
                format!("removed {name} · config only; directory kept"),
                StatusTone::Success,
            );
        }
        return vec![Effect::None];
    }
    if records_mutation_success && !state.prompt.trim().is_empty() {
        return begin_search(state);
    }
    if cmd_favorite_success && !state.prompt.trim().is_empty() {
        return begin_search(state);
    }
    if let Some(sel) = state.results.selected_id.as_deref() {
        let have_body =
            state.preview_result_id.as_deref() == Some(sel) && state.preview_body.is_some();
        let in_flight =
            state.pending_preview_id.is_some() && state.preview_result_id.as_deref() == Some(sel);
        if !have_body && !in_flight {
            return preview_effect(state);
        }
    }
    vec![Effect::None]
}

fn records_query_active(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    matches!(
        lower.split_whitespace().next(),
        Some("rec") | Some("record")
    )
}

pub fn command_recipes_query_active(prompt: &str) -> bool {
    matches!(
        prompt.split_whitespace().next(),
        Some("cmd") | Some("recipe") | Some("recipes")
    )
}

fn project_remove_name(prompt: &str) -> Option<&str> {
    let mut tokens = prompt.split_whitespace();
    let trigger = tokens.next()?.to_ascii_lowercase();
    if !matches!(trigger.as_str(), "p" | "proj" | "project") {
        return None;
    }
    if !tokens.next()?.eq_ignore_ascii_case("remove") {
        return None;
    }
    tokens.next().filter(|name| !name.is_empty())
}

fn schedule_search(state: &mut AppState) -> Vec<Effect> {
    // Cancel in-flight work immediately so typing stays responsive, but delay the
    // new Search until the quiet period so bursts don't thrash modules.
    let mut effects = cancel_active(state);
    // Keep prior results visible during debounce (clear only in begin_search).
    clear_action_ui(state);
    if state.prompt.is_empty() {
        state.search_debounce_deadline = None;
        state.results.items.clear();
        state.results.selected_id = None;
        state.preview_body = None;
        state.preview_result_id = None;
        state.pending_preview_id = None;
        state.status.set("Ready", StatusTone::Success);
        state.schedule_hub_refresh();
        effects.push(Effect::LoadHub);
        return effects;
    }
    state.hub_refresh_deadline = None;
    state.search_debounce_deadline =
        Some(std::time::Instant::now() + std::time::Duration::from_millis(80));
    state.status.set("Typing…", StatusTone::Progress);
    effects
}

fn flush_pending_search_or_continue(state: &mut AppState) -> Option<Vec<Effect>> {
    if state.search_debounce_deadline.is_some() {
        state.search_debounce_deadline = None;
        return Some(begin_search(state));
    }
    None
}

fn begin_search(state: &mut AppState) -> Vec<Effect> {
    clear_action_ui(state);
    state.search_debounce_deadline = None;
    let mut effects = cancel_active(state);
    if state.prompt.is_empty() {
        state.results.items.clear();
        state.results.selected_id = None;
        state.results.scroll = 0;
        state.status.set("Ready", StatusTone::Success);
        state.schedule_hub_refresh();
        effects.push(Effect::LoadHub);
        return effects;
    }
    state.hub_refresh_deadline = None;
    state.push_query_history(&state.prompt.clone());
    let request_id = next_request_id(state);
    state.active_request = Some(request_id.clone());
    state.request_seq_seen = 0;
    state.results.items.clear();
    state.results.selected_id = None;
    effects.push(Effect::Search {
        request_id,
        query: state.prompt.clone(),
    });
    effects
}

fn cancel_active(state: &mut AppState) -> Vec<Effect> {
    if let Some(request_id) = state.active_request.take() {
        vec![Effect::CancelSearch { request_id }]
    } else {
        Vec::new()
    }
}

fn next_request_id(state: &mut AppState) -> String {
    state.search_generation = state.search_generation.saturating_add(1);
    format!("req-{}", state.search_generation)
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::view_model::StatusTone;
    use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
    use luma_protocol::{ActionDescriptorDto, ActionOutcomeDto, Event, SearchItemDto};

    #[test]
    fn typing_schedules_search_then_flush_cancels_old() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::KeyChar('a'));
        assert!(state.search_debounce_deadline.is_some());
        assert!(!effects.iter().any(|e| matches!(e, Effect::Search { .. })));

        let effects = update(&mut state, Msg::FlushSearch);
        assert!(matches!(effects.last(), Some(Effect::Search { .. })));
        let first = state.active_request.clone().unwrap();

        let effects = update(&mut state, Msg::KeyChar('p'));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::CancelSearch { request_id } if request_id == &first)));
        assert!(!effects.iter().any(|e| matches!(e, Effect::Search { .. })));
        let effects = update(&mut state, Msg::FlushSearch);
        assert!(effects.iter().any(|e| matches!(e, Effect::Search { .. })));
    }

    #[test]
    fn late_chunk_from_old_request_is_ignored() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let _ = update(&mut state, Msg::FlushSearch);
        let active = state.active_request.clone().unwrap();

        let _ = update(&mut state, Msg::KeyChar('b'));
        let _ = update(&mut state, Msg::FlushSearch);
        let new_active = state.active_request.clone().unwrap();
        assert_ne!(active, new_active);

        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "old".into(),
                module_id: "mock".into(),
                title: "old".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(!applied);
        assert!(state.results.items.is_empty());
    }

    #[test]
    fn chunk_for_active_request_updates_results() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('x'));
        let _ = update(&mut state, Msg::FlushSearch);
        let active = state.active_request.clone().unwrap();
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: active,
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "1".into(),
                module_id: "mock".into(),
                title: "Alpha".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 10.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(applied);
        assert_eq!(state.results.items.len(), 1);
        assert_eq!(state.results.selected_id.as_deref(), Some("1"));
    }

    #[test]
    fn typing_keeps_results_during_debounce_submit_flushes_search() {
        let mut state = AppState::default();
        state.prompt = "old".into();
        let _ = update(&mut state, Msg::FlushSearch);
        let applied = state.apply_engine_event(Event::ResultsChunk {
            request_id: state.active_request.clone().unwrap(),
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: "stale".into(),
                module_id: "mock".into(),
                title: "stale hit".into(),
                subtitle: None,
                kind: "mock".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        });
        assert!(applied);
        assert_eq!(state.results.items.len(), 1);
        state.results.selected_id = Some("stale".into());

        // One more character — debounce pending; keep prior rows to avoid empty flash.
        let _ = update(&mut state, Msg::KeyChar('x'));
        assert!(state.search_debounce_deadline.is_some());
        assert_eq!(state.results.items.len(), 1);
        assert_eq!(state.status.text, "Typing…");

        let effects = update(&mut state, Msg::Submit);
        assert!(
            effects.iter().any(|e| matches!(e, Effect::Search { .. })),
            "Submit while debounce pending should flush a new search"
        );
        assert!(state.results.items.is_empty());
        assert!(state.search_debounce_deadline.is_none());
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, Effect::ExecuteAction { .. } | Effect::ListActions { .. })),
            "must not act on the stale selection"
        );
    }

    #[test]
    fn hub_digit_focuses_third_window() {
        let mut state = AppState::default();
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "all".into(),
            windows: vec![
                crate::view_model::HubWindowRow {
                    id: "win:1".into(),
                    title: "A".into(),
                },
                crate::view_model::HubWindowRow {
                    id: "win:2".into(),
                    title: "B".into(),
                },
                crate::view_model::HubWindowRow {
                    id: "win:3".into(),
                    title: "C".into(),
                },
            ],
            more: None,
            status_kind: Some("permission_required".into()),
            status_title: Some("hint".into()),
            status_subtitle: None,
        });
        let effects = update(&mut state, Msg::PickWindowDigit(3));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, .. }
            if result_id == "win:3" && action_id == "focus"
        )));
    }

    #[test]
    fn win_digit_only_when_list_focused() {
        let mut state = AppState::default();
        state.prompt = "win ".into();
        state.prompt_cursor = state.prompt_char_len();
        state.results.items.push(SearchItem {
            id: ResultId::new("win:a"),
            module_id: ModuleId::new("luma.windows"),
            title: "A".into(),
            subtitle: None,
            kind: "window".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("focus"),
                label: "Focus".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.focus = FocusZone::Prompt;
        let effects = update(&mut state, Msg::PickWindowDigit(1));
        assert_eq!(effects, vec![Effect::None]);
        state.focus = FocusZone::List;
        let effects = update(&mut state, Msg::PickWindowDigit(1));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, .. }
            if result_id == "win:a" && action_id == "focus"
        )));
    }

    #[test]
    fn help_meta_does_not_run_primary_status() {
        let mut state = AppState::default();
        state.prompt = ":help".into();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Help);
        assert_eq!(state.status.text, "help");
    }

    #[test]
    fn meta_command_does_not_require_enter_to_flush_debounce() {
        let mut state = AppState::default();
        for c in ":commands".chars() {
            let _ = update(&mut state, Msg::KeyChar(c));
        }
        assert!(state.search_debounce_deadline.is_some());

        let effects = update(&mut state, Msg::Submit);

        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Commands);
        assert!(state.prompt.is_empty());
        assert_eq!(state.overlay_restore_prompt.as_deref(), Some(":commands"));
        assert!(state.search_debounce_deadline.is_none());
    }

    #[test]
    fn esc_from_commands_restores_meta_prompt() {
        let mut state = AppState::default();
        state.prompt = ":commands".into();
        state.prompt_cursor = state.prompt_char_len();
        let _ = update(&mut state, Msg::Submit);
        assert_eq!(state.route, Route::Commands);
        assert!(state.prompt.is_empty());

        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert_eq!(state.prompt, ":commands");
        assert!(state.overlay_restore_prompt.is_none());
    }

    #[test]
    fn esc_clears_prompt_and_debounce() {
        let mut state = AppState::default();
        state.prompt = "clip hello".into();
        state.prompt_cursor = state.prompt_char_len();
        let _ = update(&mut state, Msg::KeyChar('x'));
        assert!(state.search_debounce_deadline.is_some());

        let _ = update(&mut state, Msg::Cancel);

        assert!(state.prompt.is_empty());
        assert!(state.search_debounce_deadline.is_none());
    }

    #[test]
    fn esc_to_empty_prompt_reloads_hub() {
        let mut state = AppState::default();
        state.prompt = "app safari".into();
        state.prompt_cursor = state.prompt_char_len();
        let effects = update(&mut state, Msg::Cancel);
        assert!(state.prompt.is_empty());
        assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    }

    #[test]
    fn cancel_opens_quit_confirm_from_empty_search() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::QuitConfirm);
        assert!(!state.should_quit);
    }

    #[test]
    fn browse_query_parent_pops_one_path_component() {
        assert_eq!(
            browse_query_parent("n browse /Notes/Inbox/nested"),
            Some("n browse /Notes/Inbox".into())
        );
        assert_eq!(
            browse_query_parent("proj browse /dev/app"),
            Some("proj browse /dev".into())
        );
        assert_eq!(browse_query_parent("n browse"), None);
        assert_eq!(browse_query_parent("n hello"), None);
        assert_eq!(
            browse_query_parent("n browse /Notes"),
            Some("n browse /".into())
        );
    }

    #[test]
    fn records_category_browse_uses_rec_trigger_and_category_payload() {
        let mut state = AppState::default();
        let item = SearchItem {
            id: ResultId::new("rec:cat:电影"),
            module_id: ModuleId::new("luma.records"),
            title: "电影/".into(),
            subtitle: Some("Enter to browse category".into()),
            kind: "category".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "category": "电影" })),
        };
        let effects = drill_into_browse(&mut state, &item);
        assert_eq!(state.prompt, "rec browse 电影");
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::Search { query, .. } if query == "rec browse 电影")));
    }

    #[test]
    fn records_actions_seed_prompt_for_rate_and_note() {
        let mut state = AppState::default();
        let item = SearchItem {
            id: ResultId::new("rec:42"),
            module_id: ModuleId::new("luma.records"),
            title: "沙丘".into(),
            subtitle: None,
            kind: "record".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "View".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        };
        let _ = seed_record_edit(&mut state, &item, "rate");
        assert_eq!(state.prompt, "rec rate 42 ");
        let _ = seed_record_edit(&mut state, &item, "note");
        assert_eq!(state.prompt, "rec note 42 ");
    }

    #[test]
    fn esc_pops_browse_nav_stack_then_clears_at_root() {
        let mut state = AppState::default();
        state.prompt = "n browse".into();
        state.prompt_cursor = state.prompt_char_len();
        state.results.items.push(SearchItem {
            id: ResultId::new("browse:n:/tmp/notes/Inbox"),
            module_id: ModuleId::new("luma.notes"),
            title: "Inbox/".into(),
            subtitle: Some("/tmp/notes/Inbox".into()),
            kind: "directory".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("browse:n:/tmp/notes/Inbox".into());
        let _ = update(&mut state, Msg::Submit);
        assert_eq!(state.prompt, "n browse /tmp/notes/Inbox");
        assert_eq!(state.browse_nav_stack, vec!["n browse".to_string()]);

        state.active_request = None;
        let effects = update(&mut state, Msg::Cancel);
        assert!(
            effects.iter().any(|e| matches!(e, Effect::Search { .. })),
            "expected search after browse-up: {effects:?}"
        );
        assert_eq!(state.prompt, "n browse");
        assert!(state.browse_nav_stack.is_empty());

        state.active_request = None;
        let effects = update(&mut state, Msg::Cancel);
        assert!(
            effects.iter().any(|e| matches!(e, Effect::LoadHub)),
            "expected LoadHub after clearing to hub: {effects:?}"
        );
        assert!(state.prompt.is_empty());
        assert!(!state.should_quit);
    }

    #[test]
    fn ctrl_u_clears_browse_stack_for_home() {
        let mut state = AppState::default();
        state.prompt = "n browse /tmp/notes/Inbox".into();
        state.prompt_cursor = state.prompt_char_len();
        state.browse_nav_stack = vec!["n browse".into()];
        let _ = update(&mut state, Msg::ClearToStart);
        assert!(state.prompt.is_empty());
        assert!(state.browse_nav_stack.is_empty());
    }

    #[test]
    fn prompt_cursor_inserts_in_middle() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let _ = update(&mut state, Msg::KeyChar('c'));
        let _ = update(&mut state, Msg::CursorLeft);
        let _ = update(&mut state, Msg::KeyChar('b'));
        assert_eq!(state.prompt, "abc");
        assert_eq!(state.prompt_cursor, 2);
    }

    #[test]
    fn page_down_moves_selection() {
        let mut state = AppState::default();
        for i in 0..12 {
            state.results.items.push(SearchItem {
                id: ResultId::new(format!("{i}")),
                module_id: ModuleId::new("mock"),
                title: format!("Item {i}"),
                subtitle: None,
                kind: "mock".into(),
                score: (12 - i) as f64,
                primary_action: ActionDescriptor {
                    id: ActionId::new("open"),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            });
        }
        state.results.selected_id = Some("0".into());
        let _ = update(&mut state, Msg::SelectPageDown);
        assert_eq!(state.results.selected_id.as_deref(), Some("5"));
    }

    #[test]
    fn action_picker_digit_runs_action() {
        let mut state = AppState::default();
        state.route = Route::ActionPicker;
        state.action_result_id = Some("r1".into());
        state.action_choices = vec![
            ActionDescriptorDto {
                id: "a".into(),
                label: "A".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptorDto {
                id: "b".into(),
                label: "B".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
        ];
        let effects = update(&mut state, Msg::PickActionDigit(2));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ExecuteAction { action_id, .. } if action_id == "b")),
            "{effects:?}"
        );
    }

    #[test]
    fn submit_lists_actions_for_selected_result() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("1".into());
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(
            effects,
            vec![Effect::ListActions {
                result_id: "1".into()
            }]
        );
        assert_eq!(
            state.awaiting_actions,
            Some(AwaitingActions {
                intent: ActionsIntent::Primary,
                result_id: "1".into(),
            })
        );
    }

    #[test]
    fn actions_available_enters_confirm_for_destructive() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("force"),
                label: "Force".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("1".into());
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![ActionDescriptorDto {
                    id: "force".into(),
                    label: "Force".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert!(state.pending_action.is_some());
    }

    #[test]
    fn missing_primary_action_reports_contract_violation() {
        let mut state = AppState::default();
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("1".into());
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![ActionDescriptorDto {
                    id: "open".into(),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Search);
        assert!(state.pending_action.is_none());
        assert!(state.status.text.contains("module contract violation"));
        assert_eq!(state.status.tone, StatusTone::Error);
    }

    #[test]
    fn confirm_submit_executes_with_confirmation_true() {
        let mut state = AppState::default();
        state.route = Route::ConfirmAction;
        state.pending_action = Some(PendingAction {
            result_id: "1".into(),
            action: ActionDescriptorDto {
                id: "force".into(),
                label: "Force".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        });
        let effects = update(&mut state, Msg::Submit);
        assert!(matches!(
            effects.as_slice(),
            [Effect::ExecuteAction {
                action_id,
                confirmation: true,
                ..
            }] if action_id == "force"
        ));
        assert_eq!(state.route, Route::Search);
    }

    #[test]
    fn second_execute_rejected_while_action_active() {
        let mut state = AppState::default();
        state.active_operation = Some("op-1".into());
        state.results.items = vec![SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("luma.fake"),
            title: "hit".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        }];
        state.results.selected_id = Some("1".into());
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Primary,
            result_id: "1".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![ActionDescriptorDto {
                    id: "open".into(),
                    label: "Open".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.active_operation.as_deref(), Some("op-1"));
        assert!(state.status.text.contains("already running"));
    }

    #[test]
    fn tab_opens_action_picker() {
        let mut state = AppState::default();
        state.results.selected_id = Some("1".into());
        let effects = update(&mut state, Msg::OpenActions);
        assert_eq!(
            effects,
            vec![Effect::ListActions {
                result_id: "1".into()
            }]
        );
        assert_eq!(
            state.awaiting_actions,
            Some(AwaitingActions {
                intent: ActionsIntent::Picker,
                result_id: "1".into(),
            })
        );
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "1".into(),
                actions: vec![
                    ActionDescriptorDto {
                        id: "open".into(),
                        label: "Open".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    ActionDescriptorDto {
                        id: "delete".into(),
                        label: "Delete".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                ],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ActionPicker);
        assert_eq!(state.action_choices.len(), 2);
        assert_eq!(state.action_result_id.as_deref(), Some("1"));
    }

    #[test]
    fn mismatched_actions_available_is_ignored() {
        let mut state = AppState::default();
        state.awaiting_actions = Some(AwaitingActions {
            intent: ActionsIntent::Picker,
            result_id: "A".into(),
        });
        let effects = update(
            &mut state,
            Msg::Engine(Event::ActionsAvailable {
                result_id: "B".into(),
                actions: vec![ActionDescriptorDto {
                    id: "delete".into(),
                    label: "Delete".into(),
                    risk: ActionRisk::Destructive,
                    confirmation: true,
                }],
            }),
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::Search);
        assert!(state.action_choices.is_empty());
        assert_eq!(
            state
                .awaiting_actions
                .as_ref()
                .map(|a| a.result_id.as_str()),
            Some("A")
        );
    }

    #[test]
    fn picker_submit_uses_pinned_result_id_not_selection() {
        let mut state = AppState::default();
        state.route = Route::ActionPicker;
        state.action_result_id = Some("A".into());
        state.results.selected_id = Some("B".into());
        state.action_choices = vec![ActionDescriptorDto {
            id: "delete".into(),
            label: "Delete".into(),
            risk: ActionRisk::Destructive,
            confirmation: true,
        }];
        state.action_selected = 0;
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert_eq!(
            state.pending_action.as_ref().map(|p| p.result_id.as_str()),
            Some("A")
        );
    }

    #[test]
    fn search_finished_clears_active_request_so_esc_clears_immediately() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::KeyChar('a'));
        let _ = update(&mut state, Msg::FlushSearch);
        let request_id = state.active_request.clone().expect("active request");
        let applied = state.apply_engine_event(Event::SearchFinished {
            request_id,
            total: 1,
            elapsed_ms: 3,
        });
        assert!(applied);
        assert!(state.active_request.is_none());
        state.results.items.push(SearchItem {
            id: ResultId::new("1"),
            module_id: ModuleId::new("mock"),
            title: "One".into(),
            subtitle: None,
            kind: "mock".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.prompt = "a".into();
        let effects = update(&mut state, Msg::Cancel);
        assert!(
            effects.iter().any(|e| matches!(e, Effect::LoadHub)),
            "expected LoadHub after Esc to empty: {effects:?}"
        );
        assert!(state.prompt.is_empty());
        assert!(state.results.items.is_empty());
        assert!(!state.should_quit);
    }

    #[test]
    fn quit_opens_confirm_then_enter_exits() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::Quit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::QuitConfirm);
        assert!(!state.should_quit);

        let effects = update(&mut state, Msg::Submit);
        assert!(state.should_quit);
        assert!(
            effects.is_empty()
                || effects
                    .iter()
                    .all(|e| matches!(e, Effect::None | Effect::CancelSearch { .. }))
        );
    }

    #[test]
    fn quit_confirm_esc_returns_to_search() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::Quit);
        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert!(!state.should_quit);
    }

    #[test]
    fn settings_ctrl_u_does_not_edit_prompt() {
        let mut state = AppState::default();
        state.prompt = "keep me".into();
        state.prompt_cursor = state.prompt_char_len();
        state.route = Route::Settings;
        state.settings_modules = vec![crate::view_model::SettingsModuleRow {
            id: "luma.fake".into(),
            name: "Fake".into(),
            enabled: true,
        }];
        let effects = update(&mut state, Msg::ClearToStart);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.prompt, "keep me");
        assert!(state.search_debounce_deadline.is_none());
    }

    #[test]
    fn stale_preview_loaded_is_ignored() {
        let mut state = AppState::default();
        state.results.items.push(luma_domain::SearchItem {
            id: luma_domain::ResultId::new("note:a"),
            module_id: luma_domain::ModuleId::new("luma.notes"),
            title: "a".into(),
            subtitle: None,
            kind: "note".into(),
            score: 1.0,
            primary_action: luma_domain::ActionDescriptor {
                id: luma_domain::ActionId::new("open"),
                label: "Open".into(),
                risk: luma_domain::ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("note:a".into());
        let effects = preview_effect(&mut state);
        let Effect::LoadPreview {
            preview_id: first_id,
            ..
        } = &effects[0]
        else {
            panic!("{effects:?}");
        };
        let first_id = *first_id;

        // New request for same selection after bump (simulate re-select).
        state.pending_preview_id = None;
        state.preview_body = None;
        let effects2 = preview_effect(&mut state);
        let Effect::LoadPreview {
            preview_id: second_id,
            ..
        } = &effects2[0]
        else {
            panic!("{effects2:?}");
        };
        assert_ne!(first_id, *second_id);

        let applied = state.apply_engine_event(Event::PreviewLoaded {
            result_id: "note:a".into(),
            preview_id: first_id,
            body: "STALE".into(),
        });
        assert!(!applied);
        assert_ne!(state.preview_body.as_deref(), Some("STALE"));

        let applied = state.apply_engine_event(Event::PreviewLoaded {
            result_id: "note:a".into(),
            preview_id: *second_id,
            body: "FRESH".into(),
        });
        assert!(applied);
        assert_eq!(state.preview_body.as_deref(), Some("FRESH"));
    }

    #[test]
    fn toggle_setting_uses_update_settings_cas() {
        let mut state = AppState::default();
        state.route = Route::Settings;
        state.settings_version = 3;
        state.settings_modules = vec![crate::view_model::SettingsModuleRow {
            id: "luma.fake".into(),
            name: "Fake".into(),
            enabled: true,
        }];
        let effects = update(&mut state, Msg::ToggleSetting);
        assert_eq!(
            effects,
            vec![Effect::UpdateSettings {
                module_id: "luma.fake".into(),
                enabled: false,
                expected_version: 3,
            }]
        );
        // No optimistic flip — wait for SettingsChanged.
        assert!(state.settings_modules[0].enabled);
    }

    #[test]
    fn hub_window_enter_focuses_without_prompt() {
        let mut state = AppState::default();
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "Cursor".into(),
            windows: vec![crate::view_model::HubWindowRow {
                id: "win:pid:1|num:1".into(),
                title: "Luma".into(),
            }],
            more: None,
            status_kind: None,
            status_title: None,
            status_subtitle: None,
        });
        state.hub_selected = 0;
        let effects = update(&mut state, Msg::Submit);
        assert!(state.prompt.is_empty());
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction {
                action_id,
                result_id,
                ..
            } if action_id == "focus" && result_id == "win:pid:1|num:1"
        )));
        assert!(!effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    }

    #[test]
    fn hub_window_enter_respects_active_operation() {
        let mut state = AppState::default();
        state.active_operation = Some("op-1".into());
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "Cursor".into(),
            windows: vec![crate::view_model::HubWindowRow {
                id: "win:pid:1|num:1".into(),
                title: "Luma".into(),
            }],
            more: None,
            status_kind: None,
            status_title: None,
            status_subtitle: None,
        });
        state.hub_selected = 0;
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.active_operation.as_deref(), Some("op-1"));
    }

    #[test]
    fn empty_prompt_schedules_load_hub() {
        let mut state = AppState::default();
        state.prompt = "app x".into();
        state.prompt_cursor = state.prompt_char_len();
        let effects = update(&mut state, Msg::ClearToStart);
        assert!(state.prompt.is_empty());
        assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    }

    #[test]
    fn hub_more_row_opens_win_trigger() {
        let mut state = AppState::default();
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "Cursor".into(),
            windows: vec![crate::view_model::HubWindowRow {
                id: "win:pid:1|num:1".into(),
                title: "Luma".into(),
            }],
            more: Some(3),
            status_kind: None,
            status_title: None,
            status_subtitle: None,
        });
        state.hub_selected = 1; // more row
        let _effects = update(&mut state, Msg::Submit);
        assert_eq!(state.prompt, "win ");
    }

    #[test]
    fn hub_status_row_opens_win() {
        let mut state = AppState::default();
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "Windows".into(),
            windows: vec![],
            more: None,
            status_kind: Some("permission_required".into()),
            status_title: Some("Permission required (accessibility)".into()),
            status_subtitle: Some("Grant Accessibility".into()),
        });
        state.hub_selected = 0;
        let _ = update(&mut state, Msg::Submit);
        assert_eq!(state.prompt, "win ");
    }

    #[test]
    fn seed_config_primary_skips_action_picker() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
        let mut state = AppState::default();
        let item = SearchItem {
            id: ResultId::new("notes:configure"),
            module_id: ModuleId::new("luma.notes"),
            title: "Configure".into(),
            subtitle: Some("hint".into()),
            kind: "not_configured".into(),
            score: 0.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: Some("seed_config".into()),
            action_payload: None,
        };
        state.results.items = vec![item.clone()];
        state.results.selected_id = Some(item.id.as_str().to_string());
        let effects = request_primary_actions(&mut state);
        assert!(
            effects
                .iter()
                .all(|e| !matches!(e, Effect::ListActions { .. })),
            "seed_config primary should not open action picker: {effects:?}"
        );
        assert!(state.status.text.contains("luma config set"));
    }

    #[test]
    fn projects_without_imports_show_import_guidance() {
        use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
        let mut state = AppState::default();
        let item = SearchItem {
            id: ResultId::new("proj:not-configured"),
            module_id: ModuleId::new("luma.projects"),
            title: "No imported projects".into(),
            subtitle: Some("proj add /path".into()),
            kind: "not_configured".into(),
            score: 0.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: Some("seed_config".into()),
            action_payload: None,
        };
        state.results.items = vec![item.clone()];
        state.results.selected_id = Some(item.id.as_str().into());
        let _ = request_primary_actions(&mut state);
        assert!(state.status.text.contains("proj add"));
        assert!(!state.status.text.contains("--projects-root"));
    }

    #[test]
    fn hub_rows_order_window_then_module() {
        let mut state = AppState::default();
        state.hub_windows = Some(crate::view_model::HubWindowsState {
            app_name: "Cursor".into(),
            windows: vec![crate::view_model::HubWindowRow {
                id: "win:a".into(),
                title: "A".into(),
            }],
            more: None,
            status_kind: None,
            status_title: None,
            status_subtitle: None,
        });
        state.module_catalog = vec![crate::view_model::ModuleCatalogEntry {
            id: "luma.apps".into(),
            display_name: "Apps".into(),
            enabled: true,
            glyph: None,
            suggested_query: Some("app ".into()),
            empty_hint: None,
            supports_browse: false,
            triggers: vec![],
        }];
        let rows = state.hub_rows();
        assert_eq!(rows[0].0, "window");
        assert!(rows.iter().any(|(k, ..)| k == "module"));
    }

    #[test]
    fn hub_loaded_clamps_selection() {
        let mut state = AppState::default();
        state.hub_selected = 50;
        state.hub_scroll = 40;
        state.module_catalog = vec![crate::view_model::ModuleCatalogEntry {
            id: "luma.apps".into(),
            display_name: "Apps".into(),
            enabled: true,
            glyph: None,
            suggested_query: Some("app ".into()),
            empty_hint: None,
            supports_browse: false,
            triggers: vec![],
        }];
        let _ = state.apply_engine_event(Event::HubLoaded {
            windows: Some(luma_protocol::HubWindowsDto {
                app_name: "Cursor".into(),
                windows: vec![luma_protocol::HubWindowDto {
                    id: "win:a".into(),
                    title: "A".into(),
                }],
                more: None,
                status: None,
            }),
        });
        assert!(state.hub_selected < state.hub_rows().len());
        assert!(state.hub_scroll <= state.hub_selected);
    }

    #[test]
    fn refresh_hub_loads_when_showing_hub() {
        let mut state = AppState::default();
        assert!(state.showing_hub());
        let effects = update(&mut state, Msg::RefreshHub);
        assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
        assert!(state.hub_refresh_deadline.is_some());
        assert!(!state.dirty);
    }

    #[test]
    fn focus_gained_reloads_hub() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::FocusGained);
        assert!(effects.iter().any(|e| matches!(e, Effect::LoadHub)));
    }

    #[test]
    fn refresh_hub_skips_when_not_on_hub() {
        let mut state = AppState::default();
        state.prompt = "app ".into();
        state.hub_refresh_deadline = Some(std::time::Instant::now());
        let effects = update(&mut state, Msg::RefreshHub);
        assert!(!effects.iter().any(|e| matches!(e, Effect::LoadHub)));
        assert!(state.hub_refresh_deadline.is_none());
    }

    fn sample_wordbook_review(words: Vec<(i64, &str)>) -> AppState {
        let mut state = AppState::default();
        state.route = Route::WordbookReview;
        state.wordbook_review = Some(crate::view_model::WordbookReviewState {
            words: words
                .into_iter()
                .map(|(id, term)| crate::view_model::WordbookReviewWord {
                    id,
                    term: term.into(),
                    phonetic: String::new(),
                    meaning: format!("meaning-{term}"),
                    example: String::new(),
                })
                .collect(),
            index: 0,
            revealed: false,
            stats: crate::view_model::WordbookReviewStats {
                queue: "due".into(),
                due: 2,
                goal: 20,
                reviewed_today: 7,
                remaining_goal: 13,
                ..Default::default()
            },
            finished: false,
            pending_grade: None,
        });
        state
    }

    #[test]
    fn wordbook_review_starts_from_prompt() {
        let mut state = AppState::default();
        state.prompt = "wb review due".into();
        state.prompt_cursor = state.prompt_char_len();
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(state.route, Route::WordbookReview);
        assert!(state.prompt.is_empty());
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::LoadWordbookReview { queue } if queue == "due"
        )));
    }

    #[test]
    fn wordbook_grade_blocks_before_reveal() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let effects = update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "known".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.wordbook_review.as_ref().unwrap().index, 0);
    }

    #[test]
    fn wordbook_reveal_then_known_advances() {
        let mut state = sample_wordbook_review(vec![(1, "alpha"), (2, "beta")]);
        let _ = update(&mut state, Msg::WordbookReveal);
        assert!(state.wordbook_review.as_ref().unwrap().revealed);
        let effects = update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "known".into(),
            },
        );
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, .. }
            if result_id == "wb:1" && action_id == "known"
        )));
        state.active_operation = Some("op-1".into());
        let _ = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Success {
                message: Some("ok".into()),
            },
        });
        let review = state.wordbook_review.as_ref().unwrap();
        assert_eq!(review.index, 1);
        assert!(!review.revealed);
        assert_eq!(review.stats.session_known, 1);
        assert_eq!(review.stats.reviewed_today, 7);
        let _ = state.apply_engine_event(Event::WordbookReviewStatsUpdated {
            stats: luma_protocol::WordbookStatsDto {
                due: 2,
                new_count: 0,
                wrong: 0,
                goal: 20,
                reviewed_today: 8,
                remaining_goal: 12,
            },
        });
        assert_eq!(
            state.wordbook_review.as_ref().unwrap().stats.reviewed_today,
            8
        );
    }

    #[test]
    fn wordbook_skip_advances_without_action() {
        let mut state = sample_wordbook_review(vec![(1, "alpha"), (2, "beta")]);
        let effects = update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "skip".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        let review = state.wordbook_review.as_ref().unwrap();
        assert_eq!(review.index, 1);
        assert_eq!(review.stats.session_skipped, 1);
    }

    #[test]
    fn wordbook_skip_completion_sets_done_status() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let _ = update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "skip".into(),
            },
        );
        assert!(state.wordbook_review.as_ref().unwrap().finished);
        assert_eq!(state.status.tone, StatusTone::Success);
        assert!(state.status.text.starts_with("review done"));
    }

    #[test]
    fn help_cancel_from_hub_restores_ready_status() {
        let mut state = AppState::default();
        let _ = update(&mut state, Msg::OpenHelp);
        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert!(state.showing_hub());
        assert_eq!(state.status.text, "Ready");
    }

    #[test]
    fn project_remove_refresh_keeps_success_feedback_and_clears_row() {
        let mut state = AppState::default();
        state.prompt = "proj remove files".into();
        state.prompt_cursor = state.prompt_char_len();
        state.results.items.push(SearchItem {
            id: ResultId::new("proj:remove:files"),
            module_id: ModuleId::new("luma.projects"),
            title: "Remove files".into(),
            subtitle: Some("config only".into()),
            kind: "command".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("remove_project"),
                label: "Remove".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.selected_id = Some("proj:remove:files".into());
        state.active_operation = Some("op-remove".into());

        let settings = Event::SettingsChanged {
            version: 2,
            settings: serde_json::json!({
                "modules": [],
                "projects_roots": [],
                "imported_projects": []
            }),
        };
        let effects = apply_engine(&mut state, settings);
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::Search { .. })));
        assert!(state.results.items.is_empty());

        let _ = apply_engine(
            &mut state,
            Event::ActionFinished {
                operation_id: "op-remove".into(),
                outcome: luma_protocol::ActionOutcomeDto::Success {
                    message: Some("settings updated".into()),
                },
            },
        );
        assert!(state.status.text.contains("directory kept"));
        assert_eq!(state.status.tone, StatusTone::Success);
    }

    #[test]
    fn wordbook_mastered_requires_confirm() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        let _ = update(&mut state, Msg::WordbookReveal);
        let effects = update(
            &mut state,
            Msg::WordbookGrade {
                action_id: "mastered".into(),
            },
        );
        assert_eq!(effects, vec![Effect::None]);
        assert_eq!(state.route, Route::ConfirmAction);
        assert!(state.pending_action.is_some());
        let confirm_effects = update(&mut state, Msg::Submit);
        assert!(confirm_effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, action_id, confirmation: true, .. }
            if result_id == "wb:1" && action_id == "mastered"
        )));
        assert_eq!(state.route, Route::WordbookReview);
    }

    #[test]
    fn wordbook_esc_exits_review() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        state.overlay_restore_prompt = Some("wb review".into());
        let _ = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::Search);
        assert!(state.wordbook_review.is_none());
        assert_eq!(state.prompt, "wb review");
    }

    #[test]
    fn wordbook_esc_cancels_active_grade_before_exiting() {
        let mut state = sample_wordbook_review(vec![(1, "alpha")]);
        state.active_operation = Some("op-1".into());
        state.wordbook_review.as_mut().unwrap().pending_grade = Some("known".into());
        let effects = update(&mut state, Msg::Cancel);
        assert_eq!(state.route, Route::WordbookReview);
        assert_eq!(
            effects,
            vec![Effect::CancelOperation {
                operation_id: "op-1".into()
            }]
        );

        let applied = state.apply_engine_event(Event::ActionFinished {
            operation_id: "op-1".into(),
            outcome: ActionOutcomeDto::Cancelled,
        });
        assert!(applied);
        assert!(state
            .wordbook_review
            .as_ref()
            .unwrap()
            .pending_grade
            .is_none());
        assert_eq!(state.route, Route::WordbookReview);
    }

    #[test]
    fn wordbook_review_starts_from_search_result() {
        let mut state = AppState::default();
        state.prompt = "wb review".into();
        state.prompt_cursor = state.prompt_char_len();
        state.results.items.push(SearchItem {
            id: ResultId::new("wb:review:due"),
            module_id: ModuleId::new("luma.wordbook"),
            title: "Start review (due)".into(),
            subtitle: None,
            kind: "command".into(),
            score: 100.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("start_review"),
                label: "Start review".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "queue": "due" })),
        });
        state.results.selected_id = Some("wb:review:due".into());
        let effects = update(&mut state, Msg::Submit);
        assert_eq!(state.route, Route::WordbookReview);
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::LoadWordbookReview { queue } if queue == "due"
        )));
    }

    #[test]
    fn win_digit_ignores_non_window_rows() {
        let mut state = AppState::default();
        state.prompt = "win ".into();
        state.prompt_cursor = state.prompt_char_len();
        state.focus = FocusZone::List;
        state.results.items.push(SearchItem {
            id: ResultId::new("win:status"),
            module_id: ModuleId::new("luma.windows"),
            title: "Permission".into(),
            subtitle: None,
            kind: "permission_required".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        state.results.items.push(SearchItem {
            id: ResultId::new("win:a"),
            module_id: ModuleId::new("luma.windows"),
            title: "A".into(),
            subtitle: None,
            kind: "window".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("focus"),
                label: "Focus".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        });
        let effects = update(&mut state, Msg::PickWindowDigit(1));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::ExecuteAction { result_id, .. } if result_id == "win:a"
        )));
    }
}
