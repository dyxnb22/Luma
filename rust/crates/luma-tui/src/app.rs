use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::{command_recipes_query_active, update};
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route, StatusTone};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::{CommandRunnerPort, EnginePort};
use luma_domain::RecipeRunOutcome;
use luma_protocol::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Interactive TUI entry. Composition root (`bins/luma`) supplies the engine port.
pub async fn run_tui_with_engine(
    engine: Arc<dyn EnginePort>,
    command_runner: Arc<dyn CommandRunnerPort>,
) -> std::io::Result<()> {
    install_panic_hook();
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState::default();
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.term_width = width;
        state.term_height = height;
        state.sync_results_viewport();
    }
    state.status.set("Starting…", StatusTone::Progress);
    state.dirty = true;

    guard.terminal_mut().draw(|f| render(f, &state))?;
    state.dirty = false;

    let mut engine_rx = engine.subscribe();
    let engine_start = engine.clone();
    tokio::spawn(async move {
        let _ = engine_start.submit(Command::StartSession).await;
    });

    loop {
        if let Some(plan) = state.pending_recipe_run.take() {
            run_recipe_in_terminal(
                &mut guard,
                command_runner.as_ref(),
                engine.clone(),
                &mut state,
                plan,
            );
            if command_recipes_query_active(&state.prompt) && !state.prompt.trim().is_empty() {
                let effects = update(&mut state, Msg::FlushSearch);
                for effect in effects {
                    dispatch_effect(engine.clone(), effect);
                }
            }
        }

        if state.dirty {
            guard.terminal_mut().draw(|f| render(f, &state))?;
            state.dirty = false;
        }

        let poll_timeout = Duration::from_millis(33);
        let mut msgs: Vec<Msg> = Vec::new();
        let mut broadcast_lagged = false;

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
                dispatch_effect(engine.clone(), effect);
            }
        }

        if state.should_quit {
            break;
        }
    }

    drop(guard);
    let _ = engine.submit(Command::ShutdownSession).await;
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}

fn run_recipe_in_terminal(
    guard: &mut TerminalGuard,
    runner: &dyn CommandRunnerPort,
    engine: Arc<dyn EnginePort>,
    state: &mut AppState,
    plan: luma_domain::RecipeRunPlan,
) {
    guard.suspend();
    println!(
        "\n=== Recipe: {} ({}) ===",
        plan.recipe_title, plan.recipe_id
    );
    println!("Risk: {}", plan.risk.as_str());
    println!("Working directory: {}", plan.working_dir.display());
    println!(
        "Variant: {} — {}",
        plan.variant_id, plan.variant_description
    );

    let cancel = CancellationToken::new();
    let mut outcome = RecipeRunOutcome::Success;

    for step in &plan.steps {
        if cancel.is_cancelled() {
            outcome = RecipeRunOutcome::Cancelled;
            break;
        }
        println!("\n→ {}", step.label);
        let result = runner.run_step(step, &cancel);
        if let Some(code) = result.exit_code {
            println!("exit code: {code}");
            if code != 0 && !step.continue_on_error {
                outcome = RecipeRunOutcome::Failed;
                break;
            }
        } else if result.started {
            println!("exit code: (signal)");
            outcome = RecipeRunOutcome::Failed;
            break;
        } else {
            println!(
                "failed to start: {}",
                result.message.unwrap_or_else(|| "unknown error".into())
            );
            outcome = RecipeRunOutcome::Failed;
            break;
        }
    }

    println!("\n=== Recipe finished ===\n");
    if let Err(err) = guard.resume() {
        state
            .status
            .set(format!("terminal resume failed: {err}"), StatusTone::Error);
    } else {
        let tone = match outcome {
            RecipeRunOutcome::Success => StatusTone::Success,
            RecipeRunOutcome::Failed => StatusTone::Error,
            RecipeRunOutcome::Cancelled => StatusTone::Warning,
        };
        state
            .status
            .set(format!("recipe {} finished", plan.recipe_id), tone);
    }
    state.dirty = true;
    let recipe_id = plan.recipe_id.clone();
    let now = now_unix();
    tokio::spawn(async move {
        let _ = engine
            .submit(Command::RecordRecipeRun {
                recipe_id,
                result: outcome,
                now_unix: now,
            })
            .await;
    });
}

fn map_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    use crate::view_model::FocusZone;

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Quit,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('k') if matches!(state.route, Route::Search) => Msg::OpenActions,
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
        KeyCode::Char(c)
            if matches!(state.route, Route::Search)
                && state.focus != FocusZone::Prompt
                && state.command_recipes_selected()
                && matches!(c, 'r' | 'c' | 'f') =>
        {
            let action_id = match c {
                'r' => "run",
                'c' => "copy",
                'f' => "favorite",
                _ => "run",
            };
            Msg::RecipeShortcut {
                action_id: action_id.into(),
            }
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
        Effect::RecordRecipeRun {
            recipe_id,
            result,
            now_unix,
        } => {
            tokio::spawn(async move {
                let _ = engine
                    .submit(Command::RecordRecipeRun {
                        recipe_id,
                        result,
                        now_unix,
                    })
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
        let (tx, _rx) = tokio::sync::broadcast::channel::<u32>(2);
        let mut lagged_rx = tx.subscribe();
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
        assert!(!got.is_empty());
    }
}
