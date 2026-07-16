//! Timers — stopwatch (count-up) and countdown / Pomodoro.
//!
//! In-process only (no daemon): a 1s poller fires speech alerts while Luma is running.
//! On teardown, running timers are paused so state stays honest across restarts.

use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, LumaModule, ModuleManifest, ModuleState, SearchMode,
    SearchSink, SpeechPort, TimerEntry, TimersRepository, WarmupContext, WorkbenchMeta,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

mod format;
mod mutate;
mod parse;
mod poller;

use format::{format_hms, primary_for, timer_subtitle, timer_title};
use mutate::{payload_str, payload_u64, timer_id_from_item};
use parse::{
    parse_countdown_spec, parse_minutes_token, parse_stopwatch_name, DEFAULT_POMO_MINUTES,
};

pub struct TimersModule {
    manifest: ModuleManifest,
    store: Arc<dyn TimersRepository>,
    clock: Arc<dyn ClockPort>,
    speech: Arc<dyn SpeechPort>,
    index: Arc<RwLock<Vec<TimerEntry>>>,
    store_error: Arc<RwLock<Option<String>>>,
    /// Bumped on teardown so in-flight poller ticks cannot resurrect the index.
    refresh_generation: Arc<AtomicU64>,
    poll_cancel: Mutex<Option<CancellationToken>>,
    poll_handle: Mutex<Option<JoinHandle<()>>>,
}

impl TimersModule {
    pub fn with_deps(
        store: Arc<dyn TimersRepository>,
        clock: Arc<dyn ClockPort>,
        speech: Arc<dyn SpeechPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.timers"),
                display_name: "Timers".into(),
                triggers: vec!["tm".into(), "timer".into(), "timers".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: WorkbenchMeta {
                    glyph: Some("T".into()),
                    suggested_query: Some("tm ".into()),
                    empty_hint: Some(
                        "tm · tm pomo [min] [name] · tm sw [name] · start/pause/resume".into(),
                    ),
                    supports_browse: false,
                },
            },
            store,
            clock,
            speech,
            index: Arc::new(RwLock::new(Vec::new())),
            store_error: Arc::new(RwLock::new(None)),
            refresh_generation: Arc::new(AtomicU64::new(0)),
            poll_cancel: Mutex::new(None),
            poll_handle: Mutex::new(None),
        }
    }
}

#[async_trait]
impl LumaModule for TimersModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        match self.refresh_index().await {
            Ok(()) => {
                if !ctx.cancel.is_cancelled() {
                    self.start_poller(ctx.cancel).await;
                }
                ModuleState::Ready
            }
            Err(err) => ModuleState::Failed(err),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_normalized();
        let rest_raw = query.rest_raw();

        if let Some(err) = self.store_error.read().await.clone() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "tm:unavailable".into(),
                        module_id: "luma.timers".into(),
                        title: "Timers store unavailable".into(),
                        subtitle: Some(crate::ux::friendly_store_error(&err)),
                        kind: "unavailable".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Unavailable".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let now = match self.now_ms() {
            Ok(n) => n,
            Err(err) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "tm:unavailable".into(),
                            module_id: "luma.timers".into(),
                            title: "Clock unavailable".into(),
                            subtitle: Some(err),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };

        let mut upserts = Vec::new();

        // Create rows
        if rest.starts_with("pomo")
            || rest.starts_with("pomodoro")
            || rest.starts_with("cd ")
            || rest == "cd"
            || rest.starts_with("countdown")
            || parse_minutes_token(rest.split_whitespace().next().unwrap_or("")).is_some()
        {
            if let Some((minutes, name)) = parse_countdown_spec(&rest) {
                let duration_ms = (minutes as i64) * 60_000;
                upserts.push(SearchItemDto {
                    id: format!("tm:create:cd:{minutes}"),
                    module_id: "luma.timers".into(),
                    title: format!("Start {name} ({minutes}m)"),
                    subtitle: Some("countdown · Enter to start".into()),
                    kind: "create".into(),
                    score: 95.0,
                    primary_action_id: "create_countdown".into(),
                    primary_action_label: "Start".into(),
                    action_payload: Some(serde_json::json!({
                        "name": name,
                        "duration_ms": duration_ms,
                    })),
                    ..Default::default()
                });
            }
        }

        if rest.starts_with("sw")
            || rest.starts_with("stopwatch")
            || rest.starts_with("start")
            || rest == "sw"
            || rest == "stopwatch"
            || rest == "start"
        {
            let name = parse_stopwatch_name(rest_raw);
            upserts.push(SearchItemDto {
                id: "tm:create:sw".into(),
                module_id: "luma.timers".into(),
                title: format!("Start {name}"),
                subtitle: Some("stopwatch · Enter to start".into()),
                kind: "create".into(),
                score: 94.0,
                primary_action_id: "create_stopwatch".into(),
                primary_action_label: "Start".into(),
                action_payload: Some(serde_json::json!({ "name": name })),
                ..Default::default()
            });
        }

        let needle = if rest.starts_with("pomo")
            || rest.starts_with("pomodoro")
            || rest == "cd"
            || rest.starts_with("cd ")
            || rest.starts_with("countdown")
            || rest == "sw"
            || rest.starts_with("sw ")
            || rest.starts_with("stopwatch")
            || rest == "start"
            || rest.starts_with("start ")
            || parse_minutes_token(rest.split_whitespace().next().unwrap_or("")).is_some()
        {
            String::new()
        } else {
            rest.clone()
        };

        let timers = self.index.read().await.clone();
        for entry in timers {
            if cancel.is_cancelled() {
                return;
            }
            if !needle.is_empty()
                && !entry.name.to_lowercase().contains(&needle)
                && !entry.state.contains(&needle)
                && !entry.kind.contains(&needle)
            {
                continue;
            }
            let (action_id, action_label) = primary_for(&entry);
            upserts.push(SearchItemDto {
                id: format!("tm:{}", entry.id),
                module_id: "luma.timers".into(),
                title: timer_title(&entry, now),
                subtitle: Some(timer_subtitle(&entry)),
                kind: "timer".into(),
                score: match entry.state.as_str() {
                    "running" => 90.0,
                    "paused" => 80.0,
                    "completed" => 70.0,
                    _ => 60.0,
                },
                primary_action_id: action_id.into(),
                primary_action_label: action_label.into(),
                action_payload: Some(serde_json::json!({ "timer_id": entry.id })),
                ..Default::default()
            });
        }

        if upserts.is_empty() || rest.is_empty() {
            let empty = self.index.read().await.is_empty();
            upserts.push(SearchItemDto {
                id: "tm:help".into(),
                module_id: "luma.timers".into(),
                title: if empty {
                    "No timers yet".into()
                } else {
                    "New timer".into()
                },
                subtitle: Some(
                    "tm pomo [min] [name] · tm sw [name] · tm 25 · alerts while Luma runs".into(),
                ),
                kind: if empty {
                    "onboarding".into()
                } else {
                    "status".into()
                },
                score: if empty { 50.0 } else { 5.0 },
                primary_action_id: "noop".into(),
                primary_action_label: "Hint".into(),
                ..Default::default()
            });
        }

        if !upserts.is_empty() {
            upserts.truncate(query.limit);
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str().starts_with("tm:create:cd")
            || result.primary_action.id.as_str() == "create_countdown"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("create_countdown"),
                label: "Start".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str() == "tm:create:sw"
            || result.primary_action.id.as_str() == "create_stopwatch"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("create_stopwatch"),
                label: "Start".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str() == "tm:unavailable"
            || result.id.as_str() == "tm:help"
            || result.kind == "unavailable"
            || result.kind == "onboarding"
            || result.kind == "status"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: if result.kind == "unavailable" {
                    "Unavailable".into()
                } else {
                    "Hint".into()
                },
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }

        let Some(id) = timer_id_from_item(result) else {
            return vec![];
        };
        let entry = self.index.read().await.iter().find(|t| t.id == id).cloned();
        let Some(entry) = entry else {
            return vec![];
        };
        let (pid, plabel) = primary_for(&entry);
        let mut actions = vec![ActionDescriptor {
            id: ActionId::new(pid),
            label: plabel.into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }];
        if (entry.state == "running" || entry.state == "paused" || entry.state == "completed")
            && pid != "reset"
        {
            actions.push(ActionDescriptor {
                id: ActionId::new("reset"),
                label: "Reset".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            });
        }
        if entry.state == "running" && pid != "pause" {
            actions.push(ActionDescriptor {
                id: ActionId::new("pause"),
                label: "Pause".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            });
        }
        if entry.state == "paused" && pid != "resume" {
            actions.push(ActionDescriptor {
                id: ActionId::new("resume"),
                label: "Resume".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            });
        }
        actions.push(ActionDescriptor {
            id: ActionId::new("delete"),
            label: "Delete".into(),
            risk: ActionRisk::Destructive,
            confirmation: true,
        });
        actions
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if let Some(id) = timer_id_from_item(result) {
            let now = self.now_ms().ok()?;
            let entry = self
                .index
                .read()
                .await
                .iter()
                .find(|t| t.id == id)
                .cloned()?;
            let mut lines = vec![
                format!("Name: {}", entry.name),
                format!("Kind: {}", entry.kind),
                format!("State: {}", entry.state),
                format!("Elapsed: {}", format_hms(entry.elapsed_ms(now))),
            ];
            if let Some(left) = entry.remaining_ms(now) {
                lines.push(format!("Remaining: {}", format_hms(left)));
            }
            if let Some(d) = entry.duration_ms {
                lines.push(format!("Duration: {}", format_hms(d)));
            }
            lines.push("Alerts fire while Luma is open; quitting pauses running timers.".into());
            return Some(lines.join("\n"));
        }
        result
            .subtitle
            .clone()
            .or_else(|| Some(result.title.clone()))
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "create_countdown" => {
                let name = payload_str(&action.result, "name").unwrap_or_else(|| "Pomodoro".into());
                let duration_ms = payload_u64(&action.result, "duration_ms")
                    .map(|v| v as i64)
                    .unwrap_or(DEFAULT_POMO_MINUTES as i64 * 60_000);
                match self
                    .create_and_start(&name, "countdown", Some(duration_ms))
                    .await
                {
                    Ok(entry) => ActionOutcome::Success {
                        message: Some(format!(
                            "started {} ({})",
                            entry.name,
                            format_hms(duration_ms)
                        )),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "create_stopwatch" => {
                let name =
                    payload_str(&action.result, "name").unwrap_or_else(|| "Stopwatch".into());
                match self.create_and_start(&name, "stopwatch", None).await {
                    Ok(entry) => ActionOutcome::Success {
                        message: Some(format!("started {}", entry.name)),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "start" => {
                let Some(id) = timer_id_from_item(&action.result)
                    .map(str::to_string)
                    .or_else(|| payload_str(&action.result, "timer_id"))
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected tm:<id>".into(),
                        },
                    };
                };
                self.mutate_timer(&id, |entry, now| {
                    if entry.state == "running" {
                        return Ok(());
                    }
                    if entry.state == "completed" {
                        entry.accumulated_ms = 0;
                        entry.alerted = false;
                    }
                    entry.state = "running".into();
                    entry.started_at_ms = Some(now);
                    Ok(())
                })
                .await
            }
            "pause" => {
                let Some(id) = timer_id_from_item(&action.result)
                    .map(str::to_string)
                    .or_else(|| payload_str(&action.result, "timer_id"))
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected tm:<id>".into(),
                        },
                    };
                };
                self.mutate_timer(&id, |entry, now| {
                    if entry.state != "running" {
                        return Err(FailureKind::InvalidInput {
                            field: "state".into(),
                            message: "timer is not running".into(),
                        });
                    }
                    entry.accumulated_ms = entry.elapsed_ms(now);
                    entry.started_at_ms = None;
                    entry.state = "paused".into();
                    Ok(())
                })
                .await
            }
            "resume" => {
                let Some(id) = timer_id_from_item(&action.result)
                    .map(str::to_string)
                    .or_else(|| payload_str(&action.result, "timer_id"))
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected tm:<id>".into(),
                        },
                    };
                };
                self.mutate_timer(&id, |entry, now| {
                    if entry.state != "paused" {
                        return Err(FailureKind::InvalidInput {
                            field: "state".into(),
                            message: "timer is not paused".into(),
                        });
                    }
                    entry.state = "running".into();
                    entry.started_at_ms = Some(now);
                    Ok(())
                })
                .await
            }
            "reset" => {
                let Some(id) = timer_id_from_item(&action.result)
                    .map(str::to_string)
                    .or_else(|| payload_str(&action.result, "timer_id"))
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected tm:<id>".into(),
                        },
                    };
                };
                self.mutate_timer(&id, |entry, now| {
                    entry.accumulated_ms = 0;
                    entry.started_at_ms = None;
                    entry.alerted = false;
                    entry.state = "idle".into();
                    entry.updated_at_ms = now;
                    Ok(())
                })
                .await
            }
            "delete" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(id) = timer_id_from_item(&action.result)
                    .map(str::to_string)
                    .or_else(|| payload_str(&action.result, "timer_id"))
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected tm:<id>".into(),
                        },
                    };
                };
                match self.store.delete(&id) {
                    Ok(()) => {
                        let _ = self.refresh_index().await;
                        ActionOutcome::Success {
                            message: Some("deleted".into()),
                        }
                    }
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io {
                            context: err.to_string(),
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn teardown(&self) {
        self.refresh_generation.fetch_add(1, Ordering::SeqCst);
        self.stop_poller().await;
        self.pause_all_running().await;
        *self.index.write().await = Vec::new();
        *self.store_error.write().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{ControllableClock, FakeSpeech, MemoryTimersRepository};
    use luma_test_support::collect_search_items;

    fn module_at(ms: i64) -> (TimersModule, Arc<ControllableClock>, Arc<FakeSpeech>) {
        let clock = Arc::new(ControllableClock::new("2026-01-01", ms));
        let speech = Arc::new(FakeSpeech::new());
        let m = TimersModule::with_deps(
            Arc::new(MemoryTimersRepository::new()),
            clock.clone(),
            speech.clone(),
        );
        (m, clock, speech)
    }

    #[tokio::test]
    async fn create_pause_resume_countdown() {
        let (m, clock, _) = module_at(1_000_000);
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;

        let created = m
            .create_and_start("Focus", "countdown", Some(60_000))
            .await
            .unwrap();
        assert_eq!(created.state, "running");

        clock.advance_ms(15_000);
        let pause = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("tm:{}", created.id)),
                        module_id: ModuleId::new("luma.timers"),
                        title: "x".into(),
                        subtitle: None,
                        kind: "timer".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("pause"),
                            label: "Pause".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("pause"),
                        label: "Pause".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(pause, ActionOutcome::Success { .. }));
        let paused = m.store.get(&created.id).unwrap().unwrap();
        assert_eq!(paused.state, "paused");
        assert_eq!(paused.accumulated_ms, 15_000);
        assert!(paused.started_at_ms.is_none());

        clock.advance_ms(5_000);
        let resume = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("tm:{}", created.id)),
                        module_id: ModuleId::new("luma.timers"),
                        title: "x".into(),
                        subtitle: None,
                        kind: "timer".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("resume"),
                            label: "Resume".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("resume"),
                        label: "Resume".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(resume, ActionOutcome::Success { .. }));
        clock.advance_ms(10_000);
        let running = m.store.get(&created.id).unwrap().unwrap();
        assert_eq!(running.elapsed_ms(clock.ms()), 25_000);
        m.teardown().await;
    }

    #[tokio::test]
    async fn poller_completes_and_speaks() {
        let (m, clock, speech) = module_at(0);
        let cancel = CancellationToken::new();
        m.warmup(WarmupContext {
            cancel: cancel.clone(),
        })
        .await;
        let entry = m
            .create_and_start("Pomodoro", "countdown", Some(2_000))
            .await
            .unwrap();
        clock.advance_ms(2_500);
        TimersModule::tick_completions(
            m.store.as_ref(),
            &m.index,
            &m.store_error,
            clock.as_ref(),
            speech.as_ref(),
            m.refresh_generation.load(Ordering::SeqCst),
            &m.refresh_generation,
        )
        .await;
        let done = m.store.get(&entry.id).unwrap().unwrap();
        assert_eq!(done.state, "completed");
        assert!(done.alerted);
        {
            let calls = speech.calls.lock().expect("lock");
            assert!(
                calls.iter().any(|(t, _)| t.contains("Pomodoro")),
                "expected speech alert, got {calls:?}"
            );
        }
        m.teardown().await;
    }

    #[tokio::test]
    async fn teardown_pauses_running() {
        let (m, clock, _) = module_at(100);
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        let entry = m.create_and_start("SW", "stopwatch", None).await.unwrap();
        clock.advance_ms(3_000);
        m.teardown().await;
        let paused = m.store.get(&entry.id).unwrap().unwrap();
        assert_eq!(paused.state, "paused");
        assert_eq!(paused.accumulated_ms, 3_000);
        assert!(paused.started_at_ms.is_none());
    }

    #[tokio::test]
    async fn search_create_pomo_row() {
        let (m, _, _) = module_at(0);
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        let items = collect_search_items(&m, Query::parse("tm pomo 25 deep", 20)).await;
        assert!(
            items.iter().any(|i| {
                i.primary_action.id.as_str() == "create_countdown"
                    && i.title.contains("deep")
                    && i.title.contains("25m")
            }),
            "got: {:?}",
            items.iter().map(|i| i.title.clone()).collect::<Vec<_>>()
        );
        m.teardown().await;
    }

    #[tokio::test]
    async fn delete_requires_confirmation() {
        let (m, _, _) = module_at(0);
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        let entry = m.create_and_start("X", "stopwatch", None).await.unwrap();
        let denied = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new(format!("tm:{}", entry.id)),
                        module_id: ModuleId::new("luma.timers"),
                        title: "x".into(),
                        subtitle: None,
                        kind: "timer".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("delete"),
                            label: "Delete".into(),
                            risk: ActionRisk::Destructive,
                            confirmation: true,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("delete"),
                        label: "Delete".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        m.teardown().await;
    }
}
