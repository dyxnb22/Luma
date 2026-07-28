use super::*;
use unicode_segmentation::UnicodeSegmentation;

/// Pure synchronous reducer. Must not perform I/O.
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    state.dirty = true;
    match msg {
        Msg::RecipeShortcut { action_id } => recipe_shortcut(state, &action_id),
        Msg::KeyChar(c) => {
            if state.route == Route::Commands {
                if !c.is_control() {
                    state.overlay.commands_filter.push(c);
                    state.overlay.commands_selected = 0;
                    state.status.set(
                        format!("commands · filter: {}", state.overlay.commands_filter),
                        StatusTone::Neutral,
                    );
                }
                return vec![Effect::None];
            }
            if matches!(
                state.route,
                Route::ConfirmAction | Route::ActionPicker | Route::QuitConfirm
            ) {
                return vec![Effect::None];
            }
            if matches!(state.route, Route::Help | Route::Settings | Route::Commands) {
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
        Msg::Paste(pasted) => {
            // Paste is accepted only by the searchable prompt. In particular,
            // CR/LF inside a paste must never become a confirmation or picker
            // shortcut. Search is one line, so ordinary pasted line/tab breaks
            // become one separating space; other control bytes are excluded.
            if state.route != Route::Search {
                return vec![Effect::None];
            }
            let mut text = String::with_capacity(pasted.len());
            for character in pasted.chars() {
                match character {
                    '\r' | '\n' | '\t' if !text.ends_with(' ') => text.push(' '),
                    '\r' | '\n' | '\t' => {}
                    _ if character.is_control() => {}
                    _ => text.push(character),
                }
            }
            if text.is_empty() {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.insert_prompt_text(&text);
            sync_prompt_viewport(state);
            schedule_search(state)
        }
        Msg::Backspace => {
            if state.route == Route::Commands {
                if let Some((index, _)) = state
                    .overlay
                    .commands_filter
                    .grapheme_indices(true)
                    .next_back()
                {
                    state.overlay.commands_filter.truncate(index);
                }
                state.overlay.commands_selected = 0;
                return vec![Effect::None];
            } else if state.route == Route::Help {
                dismiss_help_for_prompt_edit(state);
            } else if state.route != Route::Search {
                return vec![Effect::None];
            }
            state.focus = FocusZone::Prompt;
            state.search.history_browse = None;
            state.search.browse_nav_stack.clear();
            state.backspace_prompt();
            sync_prompt_viewport(state);
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
            sync_prompt_viewport(state);
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
                sync_prompt_viewport(state);
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
                sync_prompt_viewport(state);
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
                sync_prompt_viewport(state);
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
                sync_prompt_viewport(state);
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
            sync_prompt_viewport(state);
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
            sync_prompt_viewport(state);
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
                    match settings_patch_from_prompt(
                        &state.search.prompt,
                        &state.settings.roots.projects_roots,
                    ) {
                        Ok(Some(patch)) => {
                            state.search.debounce_deadline = None;
                            state.status.set("saving settings…", StatusTone::Progress);
                            return vec![Effect::PatchSettings {
                                patch,
                                expected_version: state.settings.version,
                            }];
                        }
                        Err(message) => {
                            state.status.set(message, StatusTone::Warning);
                            return vec![Effect::None];
                        }
                        Ok(None) => {}
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
                    if let Some(arguments) = command.strip_prefix("help ") {
                        state.search.debounce_deadline = None;
                        state.status.set(
                            format!("/help takes no arguments (got `{}`)", arguments.trim()),
                            StatusTone::Warning,
                        );
                        return vec![Effect::None];
                    }
                    if command == "commands" || command.starts_with("commands ") {
                        let filter = command
                            .strip_prefix("commands")
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        state.overlay.restore_prompt = Some(state.search.prompt.clone());
                        state.clear_prompt();
                        state.search.debounce_deadline = None;
                        let effects = open_commands(state);
                        state.overlay.commands_filter = filter;
                        return effects;
                    }
                    if command == "scroll up" || command == "scroll down" {
                        state.search.debounce_deadline = None;
                        let direction = if command.ends_with("up") {
                            ScrollDirection::Up
                        } else {
                            ScrollDirection::Down
                        };
                        state.status.set(
                            format!(
                                "Scrolled {}",
                                if direction == ScrollDirection::Up {
                                    "up"
                                } else {
                                    "down"
                                }
                            ),
                            StatusTone::Neutral,
                        );
                        return scroll_page(state, direction);
                    }
                    if command == "scroll" || command.starts_with("scroll ") {
                        state.search.debounce_deadline = None;
                        state
                            .status
                            .set("Usage: /scroll up or /scroll down", StatusTone::Warning);
                        return vec![Effect::None];
                    }
                    if command == "quit" {
                        state.search.debounce_deadline = None;
                        clear_action_ui(state);
                        state.route = Route::QuitConfirm;
                        state.status.set("Quit Luma?", StatusTone::Warning);
                        return vec![Effect::None];
                    }
                    if command.starts_with("quit ") {
                        state.search.debounce_deadline = None;
                        state
                            .status
                            .set("/quit takes no arguments", StatusTone::Warning);
                        return vec![Effect::None];
                    }
                }
                if state.incomplete_slash_trigger().is_some() {
                    state.search.prompt.push(' ');
                    state.search.prompt_cursor = state.prompt_char_len();
                    return begin_search(state);
                }
                if let Some(effects) = flush_pending_search_or_continue(state) {
                    return effects;
                }
                if state.command_recipes_selected() && state.focus != FocusZone::Prompt {
                    state.preview.hidden = false;
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
                sync_prompt_viewport(state);
                schedule_search(state)
            } else {
                vec![Effect::None]
            }
        }
        Msg::HistoryNewer => {
            if state.route == Route::Search {
                state.focus = FocusZone::Prompt;
                state.history_newer();
                sync_prompt_viewport(state);
                schedule_search(state)
            } else {
                vec![Effect::None]
            }
        }
        Msg::SelectNext => select_next_msg(state),
        Msg::SelectPrev => select_prev_msg(state),
        Msg::SelectPageUp => scroll_page(state, ScrollDirection::Up),
        Msg::SelectPageDown => scroll_page(state, ScrollDirection::Down),
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
                state.preview.hidden = !state.preview.hidden;
                state.sync_results_viewport();
                if state.preview.hidden {
                    if state.focus == FocusZone::Preview {
                        state.focus = FocusZone::List;
                    }
                    return vec![Effect::None];
                }
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

#[cfg(test)]
mod paste_tests {
    use super::*;

    #[test]
    fn paste_into_search_is_atomic_and_drops_control_characters() {
        let mut state = AppState::default();
        let effects = update(&mut state, Msg::Paste("alpha\r\nbeta\u{1b}[A".into()));

        assert_eq!(state.search.prompt, "alpha beta[A");
        assert_eq!(state.route, Route::Search);
        assert!(effects
            .iter()
            .all(|effect| !matches!(effect, Effect::ExecuteAction { .. })));
        assert!(state.search.debounce_deadline.is_some());
    }

    #[test]
    fn paste_on_confirm_cannot_confirm_or_change_the_prompt() {
        let mut state = AppState {
            route: Route::ConfirmAction,
            ..Default::default()
        };
        state.search.prompt = "before".into();

        let effects = update(&mut state, Msg::Paste("y\r".into()));

        assert_eq!(state.route, Route::ConfirmAction);
        assert_eq!(state.search.prompt, "before");
        assert!(matches!(effects.as_slice(), [Effect::None]));
    }
}
