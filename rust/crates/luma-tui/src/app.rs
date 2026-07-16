use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::update;
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route, StatusTone};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::run_interactive_terminal;
use luma_application::EnginePort;
use luma_protocol::Command;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Interactive TUI entry. Composition root (`bins/luma`) supplies the engine port.
pub async fn run_tui_with_engine(engine: Arc<dyn EnginePort>) -> std::io::Result<()> {
    install_panic_hook();
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState::default();
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.term_width = width;
        state.term_height = height;
        state.sync_results_viewport();
    }
    state
        .status
        .set("Starting…", crate::view_model::StatusTone::Progress);
    state.dirty = true;

    // Paint once before warmup so the shell is interactive while modules warm.
    guard.terminal_mut().draw(|f| render(f, &state))?;
    state.dirty = false;

    let mut engine_rx = engine.subscribe();
    let engine_start = engine.clone();
    tokio::spawn(async move {
        let _ = engine_start.submit(Command::StartSession).await;
    });

    loop {
        if state.dirty {
            guard.terminal_mut().draw(|f| render(f, &state))?;
            state.dirty = false;
        }

        let poll_timeout = Duration::from_millis(33);
        let mut msgs: Vec<Msg> = Vec::new();
        let mut broadcast_lagged = false;

        // Drain all available events. `Lagged` means skipped messages — continue
        // so we still receive later terminal events (SearchFinished / ActionFinished).
        loop {
            match engine_rx.try_recv() {
                Ok(ev) => msgs.push(Msg::Engine(ev)),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    warn!(skipped = n, "broadcast subscriber lagged");
                    broadcast_lagged = true;
                    continue;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        }

        if broadcast_lagged {
            msgs.push(Msg::BroadcastLagged);
        }

        if let Some(deadline) = state.search_debounce_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::FlushSearch);
            }
        }

        if let Some(deadline) = state.hub_refresh_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::RefreshHub);
            }
        }

        if event::poll(poll_timeout)? {
            match event::read()? {
                CEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    msgs.push(map_key(key.code, key.modifiers, &state));
                }
                CEvent::Resize(width, height) => msgs.push(Msg::Resize { width, height }),
                CEvent::FocusGained => msgs.push(Msg::FocusGained),
                CEvent::Paste(s) => {
                    for ch in s.chars() {
                        msgs.push(Msg::KeyChar(ch));
                    }
                }
                _ => {}
            }
        }

        for msg in msgs {
            let effects = update(&mut state, msg);
            for effect in effects {
                if !handle_effect_sync(engine.clone(), &mut guard, &mut state, effect.clone()) {
                    dispatch_effect(engine.clone(), effect);
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    // Restore terminal before awaiting shutdown so a hung teardown cannot
    // leave the user stuck in raw mode / alternate screen.
    drop(guard);
    let _ = engine.submit(Command::ShutdownSession).await;
    Ok(())
}

fn map_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    use crate::view_model::FocusZone;

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Quit,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('k') if matches!(state.route, Route::Search) => Msg::OpenActions,
            // Many terminals encode Ctrl-/ as the ASCII unit separator, which
            // crossterm reports as Ctrl-_. Accept both spellings.
            KeyCode::Char('/') | KeyCode::Char('_') | KeyCode::Char('\u{1f}')
                if matches!(state.route, Route::Search) =>
            {
                Msg::OpenCommands
            }
            KeyCode::Char('p') => {
                if state.focus == FocusZone::Prompt {
                    Msg::HistoryOlder
                } else {
                    Msg::SelectPrev
                }
            }
            KeyCode::Char('n') => {
                if state.focus == FocusZone::Prompt {
                    Msg::HistoryNewer
                } else {
                    Msg::SelectNext
                }
            }
            KeyCode::Char('u') => Msg::ClearToStart,
            KeyCode::Char('w') => Msg::DeleteWordBack,
            KeyCode::Char('a') => Msg::CursorHome,
            KeyCode::Char('e') => Msg::CursorEnd,
            _ => Msg::Tick,
        };
    }
    match code {
        KeyCode::BackTab if matches!(state.route, Route::Search) => Msg::TogglePreview,
        KeyCode::Tab if matches!(state.route, Route::Search) => Msg::FocusNext,
        KeyCode::Char('\u{1f}') if matches!(state.route, Route::Search) => Msg::OpenCommands,
        KeyCode::Char('?') if matches!(state.route, Route::Search) => Msg::OpenHelp,
        KeyCode::Char(c)
            if state.should_intercept_window_digit() && c.is_ascii_digit() && c != '0' =>
        {
            Msg::PickWindowDigit(c.to_digit(10).unwrap_or(0) as usize)
        }
        KeyCode::Char(c)
            if matches!(state.route, Route::WordbookReview)
                && matches!(c, '1' | '2' | '3' | 'm' | 'M' | 's' | 'S') =>
        {
            let action = match c {
                '1' => "known",
                '2' => "fuzzy",
                '3' => "unknown",
                'm' | 'M' => "mastered",
                _ => "skip",
            };
            Msg::WordbookGrade {
                action_id: action.into(),
            }
        }
        KeyCode::Char(c)
            if matches!(state.route, Route::ActionPicker) && c.is_ascii_digit() && c != '0' =>
        {
            Msg::PickActionDigit(c.to_digit(10).unwrap_or(0) as usize)
        }
        KeyCode::Char(' ') if matches!(state.route, Route::WordbookReview) => Msg::WordbookReveal,
        KeyCode::Char(' ') if matches!(state.route, Route::Settings) => Msg::ToggleSetting,
        KeyCode::Char(c) if matches!(state.route, Route::Search | Route::Help) => Msg::KeyChar(c),
        KeyCode::Char(_) => Msg::Tick,
        KeyCode::Backspace => Msg::Backspace,
        KeyCode::Delete => Msg::DeleteForward,
        KeyCode::Enter => Msg::Submit,
        KeyCode::Left => Msg::CursorLeft,
        KeyCode::Right => Msg::CursorRight,
        KeyCode::Home => Msg::CursorHome,
        KeyCode::End => Msg::CursorEnd,
        KeyCode::Up => Msg::SelectPrev,
        KeyCode::Down => Msg::SelectNext,
        KeyCode::PageUp => Msg::SelectPageUp,
        KeyCode::PageDown => Msg::SelectPageDown,
        KeyCode::Esc => Msg::Cancel,
        _ => Msg::Tick,
    }
}

fn handle_effect_sync(
    engine: Arc<dyn EnginePort>,
    guard: &mut TerminalGuard,
    state: &mut AppState,
    effect: Effect,
) -> bool {
    match effect {
        Effect::RunInteractiveTerminal {
            program,
            args,
            record_alias,
            operation_id,
        } => {
            run_interactive_terminal_effect(
                engine,
                guard,
                state,
                program,
                args,
                record_alias,
                operation_id,
            );
            true
        }
        _ => false,
    }
}

fn run_interactive_terminal_effect(
    engine: Arc<dyn EnginePort>,
    guard: &mut TerminalGuard,
    state: &mut AppState,
    program: String,
    args: Vec<String>,
    record_alias: Option<String>,
    operation_id: String,
) {
    let resume_err = guard.suspend().err();
    let spawn_result = if resume_err.is_none() {
        run_interactive_terminal(&program, &args)
    } else {
        Err(luma_application::InteractiveTerminalError::spawn(
            &program,
            std::io::Error::other("failed to suspend TUI"),
        ))
    };

    let (status, message, tone) = match spawn_result {
        Ok(status) => interactive_status(&program, status),
        Err(err) => (
            None,
            format!("failed to start {program}: {err}"),
            StatusTone::Error,
        ),
    };

    if let Err(err) = guard.resume() {
        state
            .status
            .set(format!("failed to restore TUI: {err}"), StatusTone::Error);
        state.dirty = true;
    } else {
        state.status.set(message, tone);
        state.dirty = true;
    }

    if state.active_operation.as_deref() == Some(operation_id.as_str()) {
        state.active_operation = None;
    }

    if let (Some(alias), Some(status)) = (record_alias, status) {
        if status.success() {
            if state.prompt.trim_start().starts_with("ssh") {
                state.search_debounce_deadline = Some(std::time::Instant::now());
            }
            let engine_record = engine.clone();
            tokio::spawn(async move {
                let _ = engine_record
                    .submit(Command::SshSessionEnded {
                        alias,
                        exit_code: status.code().unwrap_or(1),
                    })
                    .await;
            });
        }
    }
}

fn interactive_status(
    program: &str,
    status: ExitStatus,
) -> (Option<ExitStatus>, String, StatusTone) {
    if status.success() {
        (
            Some(status),
            format!("{program} exited"),
            StatusTone::Success,
        )
    } else {
        let code = status.code().unwrap_or(1);
        (
            Some(status),
            format!("{program} exited with code {code}"),
            StatusTone::Warning,
        )
    }
}

fn dispatch_effect(engine: Arc<dyn EnginePort>, effect: Effect) {
    match effect {
        Effect::Search { request_id, query } => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::Search { request_id, query }).await;
            });
        }
        Effect::CancelSearch { request_id } => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::CancelSearch { request_id }).await;
            });
        }
        Effect::LoadHub => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::LoadHub).await;
            });
        }
        Effect::LoadWordbookReview { queue } => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::LoadWordbookReview { queue }).await;
            });
        }
        Effect::RefreshWordbookReviewStats => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::RefreshWordbookReviewStats).await;
            });
        }
        Effect::GetSnapshot => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::GetSnapshot).await;
            });
        }
        Effect::LoadPreview {
            result_id,
            preview_id,
        } => {
            tokio::spawn(async move {
                let _ = engine
                    .submit(Command::LoadPreview {
                        result_id,
                        preview_id,
                    })
                    .await;
            });
        }
        Effect::ListActions { result_id } => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::ListActions { result_id }).await;
            });
        }
        Effect::ExecuteAction {
            operation_id,
            result_id,
            action_id,
            confirmation,
        } => {
            tokio::spawn(async move {
                let _ = engine
                    .submit(Command::ExecuteAction {
                        operation_id,
                        result_id,
                        action_id,
                        confirmation,
                    })
                    .await;
            });
        }
        Effect::CancelOperation { operation_id } => {
            tokio::spawn(async move {
                let _ = engine
                    .submit(Command::CancelOperation { operation_id })
                    .await;
            });
        }
        Effect::GetSettings => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::GetSettings).await;
            });
        }
        Effect::UpdateSettings {
            module_id,
            enabled,
            expected_version,
        } => {
            tokio::spawn(async move {
                let _ = engine
                    .submit(Command::UpdateSettings {
                        patch: serde_json::json!({
                            "enabled_modules": { module_id: enabled }
                        }),
                        expected_version,
                    })
                    .await;
            });
        }
        Effect::None => {}
        Effect::RunInteractiveTerminal { .. } => {
            warn!(
                "RunInteractiveTerminal reached async dispatch — should be handled synchronously"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_underscore_encoding_opens_commands() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('_'), KeyModifiers::CONTROL, &state);
        assert!(matches!(msg, Msg::OpenCommands));
    }

    #[test]
    fn ctrl_slash_control_character_opens_commands() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('\u{1f}'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::OpenCommands));
    }

    #[test]
    fn map_key_digit_routes_to_prompt_when_not_intercepting() {
        let mut state = AppState::default();
        state.prompt = "app ".into();
        state.prompt_cursor = state.prompt_char_len();
        let msg = map_key(KeyCode::Char('3'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::KeyChar('3')));
    }

    #[test]
    fn map_key_digit_routes_to_window_pick_on_hub() {
        let state = AppState::default();
        let msg = map_key(KeyCode::Char('2'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::PickWindowDigit(2)));
    }

    #[test]
    fn map_key_action_picker_digit_unchanged() {
        let state = AppState {
            route: Route::ActionPicker,
            ..Default::default()
        };
        let msg = map_key(KeyCode::Char('1'), KeyModifiers::empty(), &state);
        assert!(matches!(msg, Msg::PickActionDigit(1)));
    }

    #[test]
    fn drain_continues_after_lagged() {
        // Mirrors the TUI drain loop: Lagged must not stop draining.
        let (tx, _rx) = tokio::sync::broadcast::channel::<u32>(2);
        let mut lagged_rx = tx.subscribe();
        // Force lag by sending beyond capacity without receiving.
        for i in 0..20 {
            let _ = tx.send(i);
        }
        let mut got = Vec::new();
        loop {
            match lagged_rx.try_recv() {
                Ok(v) => got.push(v),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        }
        assert!(
            !got.is_empty(),
            "after Lagged, drain must continue and collect later events"
        );
    }
}
