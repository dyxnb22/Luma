use super::*;

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
                // Typing abandons overlay restore (Esc is the restore path).
                state.overlay.restore_prompt = None;
                state.route = Route::Search;
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.insert_prompt_char(c);
            sync_prompt_viewport(state);
            schedule_search(state)
        }
        Msg::Backspace => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            } else if state.route != Route::Search {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.backspace_prompt();
            schedule_search(state)
        }
        Msg::DeleteForward => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            } else if state.route != Route::Search {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.delete_forward_prompt();
            schedule_search(state)
        }
        Msg::CursorLeft => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            }
            if matches!(state.route, Route::Search) {
                state.focus = FocusZone::Prompt;
                state.clamp_prompt_cursor();
                state.search.prompt_cursor = state.search.prompt_cursor.saturating_sub(1);
            }
            vec![Effect::None]
        }
        Msg::CursorRight => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            }
            if matches!(state.route, Route::Search) {
                state.focus = FocusZone::Prompt;
                state.clamp_prompt_cursor();
                if state.search.prompt_cursor < state.prompt_char_len() {
                    state.search.prompt_cursor += 1;
                }
            }
            vec![Effect::None]
        }
        Msg::CursorHome => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            }
            if matches!(state.route, Route::Search) {
                state.focus = FocusZone::Prompt;
                state.search.prompt_cursor = 0;
            }
            vec![Effect::None]
        }
        Msg::CursorEnd => {
            if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            }
            if matches!(state.route, Route::Search) {
                state.focus = FocusZone::Prompt;
                state.search.prompt_cursor = state.prompt_char_len();
            }
            vec![Effect::None]
        }
        Msg::ClearToStart => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.clear_prompt_to_start();
            schedule_search(state)
        }
        Msg::DeleteWordBack => {
            if !matches!(state.route, Route::Search | Route::Help) {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
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
                if state.search.prompt.trim().is_empty()
                    && matches!(state.route, Route::Search)
                    && state.search.results.items.is_empty()
                {
                    return apply_hub_selection(state);
                }
                // Meta commands are local navigation. They must win over a pending
                // search debounce so one Enter opens the requested surface.
                if let Some(command) = explicit_command_prompt(&state.search.prompt) {
                    if let Some(queue) =
                        wordbook::wordbook_review_queue_from_prompt(&state.search.prompt)
                    {
                        return wordbook::begin_wordbook_review(state, queue);
                    }
                    if command == "settings" {
                        state.overlay.restore_prompt = Some(state.search.prompt.clone());
                        state.clear_prompt();
                        state.search.debounce_deadline = None;
                        return open_settings(state);
                    }
                    if command == "help" {
                        state.overlay.restore_prompt = Some(state.search.prompt.clone());
                        state.clear_prompt();
                        state.search.debounce_deadline = None;
                        state.route = Route::Help;
                        state.overlay.help_scroll = 0;
                        state.status.set("help", StatusTone::Neutral);
                        return vec![Effect::None];
                    }
                    if command == "commands" {
                        state.overlay.restore_prompt = Some(state.search.prompt.clone());
                        state.clear_prompt();
                        state.search.debounce_deadline = None;
                        return open_commands(state);
                    }
                }
                if let Some(effects) = flush_pending_search_or_continue(state) {
                    return effects;
                }
                if state.command_recipes_selected() && state.focus != FocusZone::Prompt {
                    state.preview.pinned = true;
                    return preview_effect(state);
                }
                request_primary_actions(state)
            }
            Route::Settings => toggle_setting(state),
            Route::Commands => run_command_selection(state),
            Route::WordbookReview => wordbook::wordbook_reveal(state),
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
                state.actions.action_selected =
                    state.actions.action_selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Help {
                state.overlay.help_scroll = state.overlay.help_scroll.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Settings {
                state.settings.selected = state.settings.selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Commands {
                state.overlay.commands_selected =
                    state.overlay.commands_selected.saturating_sub(PAGE_SIZE);
                return vec![Effect::None];
            }
            if matches!(state.route, Route::Search) {
                if state.focus == FocusZone::Preview && state.preview_visible() {
                    state.preview.scroll = state.preview.scroll.saturating_sub(PAGE_SIZE);
                    return vec![Effect::None];
                }
                if state.search.prompt.is_empty() && state.search.results.items.is_empty() {
                    state.hub.selected = state.hub.selected.saturating_sub(PAGE_SIZE);
                    state.ensure_hub_selection_visible();
                } else {
                    state.focus = FocusZone::List;
                    state.search.results.select_offset(-(PAGE_SIZE as isize));
                    state.preview.body = None;
                    state.preview.result_id = None;
                    state.preview.pending_id = None;
                    state.preview.scroll = 0;
                    return preview_effect(state);
                }
            }
            vec![Effect::None]
        }
        Msg::SelectPageDown => {
            if state.route == Route::ActionPicker {
                if !state.actions.action_choices.is_empty() {
                    state.actions.action_selected = (state.actions.action_selected + PAGE_SIZE)
                        .min(state.actions.action_choices.len() - 1);
                }
                return vec![Effect::None];
            }
            if state.route == Route::Help {
                state.overlay.help_scroll = state.overlay.help_scroll.saturating_add(PAGE_SIZE);
                return vec![Effect::None];
            }
            if state.route == Route::Settings {
                if !state.settings.modules.is_empty() {
                    state.settings.selected =
                        (state.settings.selected + PAGE_SIZE).min(state.settings.modules.len() - 1);
                }
                return vec![Effect::None];
            }
            if state.route == Route::Commands {
                state.overlay.commands_selected =
                    (state.overlay.commands_selected + PAGE_SIZE).min(COMMANDS.len() - 1);
                return vec![Effect::None];
            }
            if matches!(state.route, Route::Search) {
                if state.focus == FocusZone::Preview && state.preview_visible() {
                    state.preview.scroll = state.preview.scroll.saturating_add(PAGE_SIZE);
                    return vec![Effect::None];
                }
                if state.search.prompt.is_empty() && state.search.results.items.is_empty() {
                    let max = state.hub_rows().len().saturating_sub(1);
                    state.hub.selected = (state.hub.selected + PAGE_SIZE).min(max);
                    state.ensure_hub_selection_visible();
                } else {
                    state.focus = FocusZone::List;
                    state.search.results.select_offset(PAGE_SIZE as isize);
                    state.preview.body = None;
                    state.preview.result_id = None;
                    state.preview.pending_id = None;
                    state.preview.scroll = 0;
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
            if idx >= state.actions.action_choices.len() {
                return vec![Effect::None];
            }
            state.actions.action_selected = idx;
            submit_picker_selection(state)
        }
        Msg::PickWindowDigit(digit) => pick_window_digit(state, digit),
        Msg::WordbookReveal => wordbook::wordbook_reveal(state),
        Msg::WordbookGrade { action_id } => wordbook::wordbook_grade(state, action_id),
        Msg::WordbookReviewExit => wordbook::exit_wordbook_review(state),
        Msg::OpenHelp => {
            state.overlay.restore_prompt = Some(state.search.prompt.clone());
            state.route = Route::Help;
            state.overlay.help_scroll = 0;
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
            state.search.debounce_deadline = None;
            begin_search(state)
        }
        Msg::Resize { width, height } => {
            state.terminal.width = width;
            state.terminal.height = height;
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
                state.hub.refresh_deadline = None;
                return vec![Effect::None];
            }
            state.schedule_hub_refresh();
            vec![Effect::LoadHub]
        }
        Msg::BroadcastLagged => {
            state
                .status
                .set("Resyncing…", crate::view_model::StatusTone::Warning);
            if state.search.debounce_deadline.is_some() {
                state.search.debounce_deadline = None;
                return begin_search(state);
            }
            if state.search.active_request.is_some() || !state.search.prompt.trim().is_empty() {
                return begin_search(state);
            }
            vec![Effect::GetSnapshot]
        }
        Msg::TogglePreview => {
            if matches!(state.route, Route::Search) {
                state.preview.pinned = !state.preview.pinned;
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
