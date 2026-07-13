use crate::effect::Effect;
use crate::msg::Msg;
use crate::reducer::update;
use crate::render::render;
use crate::terminal::{install_panic_hook, TerminalGuard};
use crate::view_model::{AppState, Route};
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use luma_application::{Engine, EnginePort, ModuleRegistry};
use luma_modules::load_registry;
use luma_protocol::{Command, Event};
use std::sync::Arc;
use std::time::Duration;

pub async fn run_tui() -> std::io::Result<()> {
    let registry = load_registry()
        .map_err(|e| std::io::Error::other(format!("failed to load LumaNext registry: {e}")))?;
    run_tui_with_registry(registry).await
}

pub async fn run_tui_with_registry(registry: ModuleRegistry) -> std::io::Result<()> {
    install_panic_hook();
    let mut guard = TerminalGuard::enter()?;
    let mut state = AppState::default();

    let engine = Arc::new(Engine::new(registry));
    let mut engine_rx = engine.subscribe();
    engine.start_session().await;

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

        if event::poll(poll_timeout)? {
            match event::read()? {
                CEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    msgs.push(map_key(key.code, key.modifiers));
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
            let is_submit = matches!(msg, Msg::Submit);
            let effects = update(&mut state, msg);
            for effect in effects {
                dispatch_effect(engine.clone(), effect);
            }
            if is_submit {
                // Meta-commands (:doctor / :help) must not run the selected primary action.
                let meta = matches!(state.route, Route::Doctor | Route::Help)
                    || state.status.text == "doctor"
                    || state.status.text == "help";
                if !meta {
                    if let Some(result_id) = state.results.selected_id.clone() {
                        if let Some(item) = state
                            .results
                            .items
                            .iter()
                            .find(|i| i.id.as_str() == result_id)
                        {
                            let action_id = item.primary_action.id.as_str().to_string();
                            let eng = engine.clone();
                            let operation_id = format!("op-{}", state.search_generation);
                            tokio::spawn(async move {
                                eng.handle_command(Command::ExecuteAction {
                                    operation_id,
                                    result_id,
                                    action_id,
                                    confirmation: false,
                                })
                                .await;
                            });
                        }
                    }
                }
            }
        }

        if state.should_quit {
            engine.handle_command(Command::ShutdownSession).await;
            break;
        }
    }

    Ok(())
}

fn map_key(code: KeyCode, modifiers: KeyModifiers) -> Msg {
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Msg::Cancel,
            KeyCode::Char('l') => Msg::Redraw,
            KeyCode::Char('p') => Msg::SelectPrev,
            KeyCode::Char('n') => Msg::SelectNext,
            _ => Msg::Tick,
        };
    }
    match code {
        KeyCode::Char('?') => Msg::OpenHelp,
        KeyCode::Char(c) => Msg::KeyChar(c),
        KeyCode::Backspace => Msg::Backspace,
        KeyCode::Enter => Msg::Submit,
        KeyCode::Up => Msg::SelectPrev,
        KeyCode::Down => Msg::SelectNext,
        KeyCode::Esc => Msg::Cancel,
        _ => Msg::Tick,
    }
}

fn dispatch_effect(engine: Arc<Engine>, effect: Effect) {
    match effect {
        Effect::Search { request_id, query } => {
            tokio::spawn(async move {
                engine
                    .handle_command(Command::Search { request_id, query })
                    .await;
            });
        }
        Effect::CancelSearch { request_id } => {
            tokio::spawn(async move {
                engine
                    .handle_command(Command::CancelSearch { request_id })
                    .await;
            });
        }
        Effect::RunDoctor => {
            tokio::spawn(async move {
                engine.handle_command(Command::RunDoctor).await;
            });
        }
        Effect::ExportDiagnostics => {
            tokio::spawn(async move {
                engine.handle_command(Command::ExportDiagnostics).await;
            });
        }
        Effect::None => {}
    }
}

#[allow(dead_code)]
fn _event_ty(_: Event) {}
