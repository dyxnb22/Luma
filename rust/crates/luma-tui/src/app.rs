use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::update;
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::EnginePort;
use luma_protocol::{Command, Event};
use std::sync::Arc;
use std::time::Duration;

/// Interactive TUI entry. Composition root (`bins/luma`) supplies the engine port.
pub async fn run_tui_with_engine(engine: Arc<dyn EnginePort>) -> std::io::Result<()> {
    install_panic_hook();
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState::default();
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

        while let Ok(ev) = engine_rx.try_recv() {
            msgs.push(Msg::Engine(ev));
        }

        if let Some(deadline) = state.search_debounce_deadline {
            if std::time::Instant::now() >= deadline {
                msgs.push(Msg::FlushSearch);
            }
        }

        if event::poll(poll_timeout)? {
            match event::read()? {
                CEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    msgs.push(map_key(key.code, key.modifiers, &state));
                }
                CEvent::Resize(_, _) => msgs.push(Msg::Resize),
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
            let _ = engine.submit(Command::ShutdownSession).await;
            break;
        }
    }

    Ok(())
}

fn map_key(code: KeyCode, modifiers: KeyModifiers, state: &AppState) -> Msg {
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Quit,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('p') => Msg::SelectPrev,
            KeyCode::Char('n') => Msg::SelectNext,
            _ => Msg::Tick,
        };
    }
    match code {
        KeyCode::Tab if matches!(state.route, Route::Search) => Msg::OpenActions,
        KeyCode::Char('?') if matches!(state.route, Route::Search) => Msg::OpenHelp,
        KeyCode::Char(c) if matches!(state.route, Route::Search | Route::Help | Route::Doctor) => {
            Msg::KeyChar(c)
        }
        KeyCode::Char(_) => Msg::Tick,
        KeyCode::Backspace => Msg::Backspace,
        KeyCode::Enter => Msg::Submit,
        KeyCode::Up => Msg::SelectPrev,
        KeyCode::Down => Msg::SelectNext,
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
        Effect::RunDoctor => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::RunDoctor).await;
            });
        }
        Effect::ExportDiagnostics => {
            tokio::spawn(async move {
                let _ = engine.submit(Command::ExportDiagnostics).await;
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
        Effect::None => {}
    }
}

#[allow(dead_code)]
fn _event_ty(_: Event) {}
