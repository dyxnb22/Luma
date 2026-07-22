use super::*;
use crate::module::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, SearchMode, SearchSink, WarmupContext,
};
use async_trait::async_trait;
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, SearchItem};
use tokio_util::sync::CancellationToken;

struct FakeModule {
    manifest: ModuleManifest,
    wait_for_cancel: bool,
}

#[async_trait]
impl LumaModule for FakeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let item = SearchItemDto {
            id: if self.wait_for_cancel {
                "wait-1".into()
            } else {
                "fake-1".into()
            },
            module_id: self.manifest.id.as_str().to_string(),
            title: format!("Fake: {}", query.normalized),
            subtitle: None,
            kind: "fake".into(),
            score: 42.0,
            primary_action_id: "open".into(),
            primary_action_label: "Open".into(),
            ..Default::default()
        };
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: "ignored".into(),
                sequence: 1,
                upserts: vec![item],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn perform(&self, _action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if self.wait_for_cancel {
            cancel.cancelled().await;
            return ActionOutcome::Cancelled;
        }
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        ActionOutcome::Success {
            message: Some("ok".into()),
        }
    }

    async fn teardown(&self) {}
}

struct StickySearchModule {
    manifest: ModuleManifest,
    ran_after_sleep: Arc<std::sync::atomic::AtomicBool>,
}

#[async_trait]
impl LumaModule for StickySearchModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, _query: Query, sink: SearchSink, _cancel: CancellationToken) {
        // Deliberately ignore cancellation; only JoinSet abort should stop us.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        self.ran_after_sleep
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "sticky".into(),
                    module_id: "luma.sticky".into(),
                    title: "late".into(),
                    subtitle: None,
                    kind: "sticky".into(),
                    score: 1.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "Noop".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        Vec::new()
    }

    async fn perform(&self, _action: ActionRequest, _cancel: CancellationToken) -> ActionOutcome {
        ActionOutcome::Success { message: None }
    }

    async fn teardown(&self) {}
}

fn fake_registry() -> ModuleRegistry {
    let mut reg = ModuleRegistry::new();
    reg.register(Arc::new(FakeModule {
        manifest: ModuleManifest {
            id: ModuleId::new("luma.fake"),
            display_name: "Fake".into(),
            triggers: vec!["fake".into()],
            default_enabled: true,
            search_mode: SearchMode::GlobalContributing,
            required_capabilities: vec![],
            workbench: Default::default(),
        },
        wait_for_cancel: false,
    }))
    .expect("register fake");
    reg
}

#[tokio::test]
async fn search_cancel_aborts_noncooperative_module_task() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let ran = Arc::new(AtomicBool::new(false));
    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(StickySearchModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.sticky"),
                display_name: "Sticky".into(),
                triggers: vec!["sticky".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            ran_after_sleep: ran.clone(),
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    engine
        .handle_command(Command::Search {
            request_id: "sticky-1".into(),
            query: "hello".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchStarted { .. })) {}
    engine
        .handle_command(Command::CancelSearch {
            request_id: "sticky-1".into(),
        })
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(
        !ran.load(Ordering::SeqCst),
        "aborted search task must not resume after cancel"
    );
}

#[tokio::test]
async fn cancel_before_registration_is_honored() {
    let engine = Arc::new(Engine::new(fake_registry()));
    let mut events = engine.subscribe();
    engine.start_session().await;
    while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
    // Cancel arrives before Search for the same request id.
    engine
        .handle_command(Command::CancelSearch {
            request_id: "early".into(),
        })
        .await;
    assert!(matches!(
        events.recv().await,
        Ok(Event::SearchCancelled { request_id }) if request_id == "early"
    ));
    engine
        .handle_command(Command::Search {
            request_id: "early".into(),
            query: "hello".into(),
        })
        .await;
    // Must not start a live search for a pre-cancelled request.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while let Ok(ev) = events.try_recv() {
        assert!(
            !matches!(
                ev,
                Event::SearchStarted { .. } | Event::SearchFinished { .. }
            ),
            "pre-cancelled search must not start: {ev:?}"
        );
    }
}

#[tokio::test]
async fn cancel_new_search_while_previous_is_tearing_down() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let ran = Arc::new(AtomicBool::new(false));
    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(StickySearchModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.sticky"),
                display_name: "Sticky".into(),
                triggers: vec!["sticky".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            ran_after_sleep: ran.clone(),
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;

    engine
        .handle_command(Command::Search {
            request_id: "old".into(),
            query: "hello".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchStarted { .. })) {}

    let engine_b = engine.clone();
    let search_new = tokio::spawn(async move {
        engine_b
            .handle_command(Command::Search {
                request_id: "new".into(),
                query: "hello".into(),
            })
            .await;
    });
    // Let the new search acquire lifecycle and begin cancelling the old one.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    engine
        .handle_command(Command::CancelSearch {
            request_id: "new".into(),
        })
        .await;

    let mut saw_new_cancelled = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        tokio::select! {
            ev = events.recv() => {
                match ev {
                    Ok(Event::SearchCancelled { request_id }) if request_id == "new" => {
                        saw_new_cancelled = true;
                        break;
                    }
                    Ok(Event::SearchFinished { request_id, .. }) if request_id == "new" => {
                        panic!("new search must not finish after cancel");
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
        }
    }
    assert!(
        saw_new_cancelled,
        "expected SearchCancelled for new request"
    );
    search_new.await.unwrap();
    assert!(
        !ran.load(Ordering::SeqCst),
        "sticky work from cancelled searches must not complete"
    );
}

#[tokio::test]
async fn query_returns_fake_hit() {
    let (items, _events) = run_query(fake_registry(), "hello", None).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].title.contains("hello"));
}

#[tokio::test]
async fn slash_prefixed_query_targets_the_module() {
    let (items, _events) = run_query(fake_registry(), "/fake hello", None)
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].title.contains("hello"));
}

#[tokio::test]
async fn run_action_executes_fake_result() {
    let (result, outcome) = run_action(
        fake_registry(),
        "hello",
        None,
        "open",
        false,
        RunActionOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(result.id, "fake-1");
    assert!(matches!(
        outcome,
        luma_protocol::ActionOutcomeDto::Success { .. }
    ));
}

#[tokio::test]
async fn permission_failure_kind_not_empty_success() {
    let kind = FailureKind::PermissionRequired {
        capability: "ax".into(),
        guidance: "enable".into(),
    };
    assert!(kind.is_error());
    let outcome = ActionOutcome::Failed { kind };
    assert!(matches!(outcome, ActionOutcome::Failed { .. }));
}

#[tokio::test]
async fn subscribe_receives_session_ready() {
    use crate::port::EnginePort;

    let engine = Engine::new(fake_registry());
    let mut events = engine.subscribe();
    engine.start_session().await;
    assert!(matches!(
        events.recv().await,
        Ok(Event::ModuleStateChanged { .. })
    ));
    assert!(matches!(
        events.recv().await,
        Ok(Event::SessionReady { .. })
    ));
}

#[tokio::test]
async fn cancel_operation_cancels_in_flight_perform() {
    let mut registry = fake_registry();
    registry
        .register(Arc::new(FakeModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wait"),
                display_name: "Wait".into(),
                triggers: vec!["wait".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            wait_for_cancel: true,
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    engine
        .handle_command(Command::Search {
            request_id: "r1".into(),
            query: "wait hello".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}
    let execute = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .handle_command(Command::ExecuteAction {
                    operation_id: "op1".into(),
                    result_id: "wait-1".into(),
                    action_id: "open".into(),
                    confirmation: false,
                })
                .await;
        })
    };
    while !matches!(events.recv().await, Ok(Event::ActionStarted { .. })) {}
    engine
        .handle_command(Command::CancelOperation {
            operation_id: "op1".into(),
        })
        .await;
    let outcome = loop {
        if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
            break outcome;
        }
    };
    assert!(matches!(
        outcome,
        luma_protocol::ActionOutcomeDto::Cancelled
    ));
    execute.await.unwrap();
}

#[tokio::test]
async fn disable_module_cancels_in_flight_perform() {
    let mut registry = fake_registry();
    registry
        .register(Arc::new(FakeModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wait"),
                display_name: "Wait".into(),
                triggers: vec!["wait".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            wait_for_cancel: true,
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    engine
        .handle_command(Command::Search {
            request_id: "r1".into(),
            query: "wait hello".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}
    let execute = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .handle_command(Command::ExecuteAction {
                    operation_id: "op-disable".into(),
                    result_id: "wait-1".into(),
                    action_id: "open".into(),
                    confirmation: false,
                })
                .await;
        })
    };
    while !matches!(events.recv().await, Ok(Event::ActionStarted { .. })) {}
    engine
        .handle_command(Command::SetModuleEnabled {
            module_id: "luma.wait".into(),
            enabled: false,
        })
        .await;
    let outcome = loop {
        if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
            break outcome;
        }
    };
    assert!(matches!(
        outcome,
        luma_protocol::ActionOutcomeDto::Cancelled
    ));
    execute.await.unwrap();
}

#[tokio::test]
async fn start_session_skips_warmup_for_disabled_modules() {
    let mut registry = fake_registry();
    let _ = registry.set_enabled("luma.fake", false);
    let engine = Engine::new(registry);
    let mut events = engine.subscribe();
    engine.start_session().await;
    let first = events.recv().await.unwrap();
    assert!(matches!(
        first,
        Event::ModuleStateChanged { ref module_id, ref state }
            if module_id == "luma.fake" && state == "disabled"
    ));
    assert!(matches!(
        events.recv().await,
        Ok(Event::SessionReady { .. })
    ));
}

#[tokio::test]
async fn update_settings_persists_with_config_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(luma_storage::ConfigStore::with_path(
        dir.path().join("settings.toml"),
    ));
    let settings = Arc::new(crate::TomlSettingsRepository::new(store.clone()));
    let engine = Engine::with_settings(fake_registry(), Some(settings));
    let mut events = engine.subscribe();
    engine
        .handle_command(Command::UpdateSettings {
            patch: serde_json::json!({"enabled_modules": {"luma.fake": false}}),
            expected_version: 1,
        })
        .await;
    let event = loop {
        if let Ok(Event::SettingsChanged { version, settings }) = events.recv().await {
            break (version, settings);
        }
    };
    assert_eq!(event.0, 2);
    let modules = event.1["modules"].as_array().expect("modules array");
    assert!(
        modules
            .iter()
            .any(|m| m["id"] == "luma.fake" && m["enabled"] == false),
        "{modules:?}"
    );
    assert!(!store.load_or_default().unwrap().enabled_modules["luma.fake"]);
}

#[tokio::test]
async fn import_project_cas_conflict_does_not_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(luma_storage::ConfigStore::with_path(
        dir.path().join("settings.toml"),
    ));
    let settings = Arc::new(crate::TomlSettingsRepository::new(store.clone()));
    let engine = Engine::with_settings(fake_registry(), Some(settings));
    let mut events = engine.subscribe();

    let proj1 = dir.path().join("proj1");
    let proj2 = dir.path().join("proj2");
    std::fs::create_dir(&proj1).unwrap();
    std::fs::create_dir(&proj2).unwrap();

    engine
        .handle_command(Command::UpdateSettings {
            patch: serde_json::json!({"import_project": proj1.display().to_string()}),
            expected_version: 1,
        })
        .await;
    loop {
        if let Ok(Event::SettingsChanged { .. }) = events.recv().await {
            break;
        }
    }

    let mut current = store.load_or_default().unwrap();
    current.enabled_modules.insert("luma.fake".into(), false);
    store
        .update_cas(current.settings_version, current)
        .expect("bump version");

    engine
        .handle_command(Command::UpdateSettings {
            patch: serde_json::json!({"import_project": proj2.display().to_string()}),
            expected_version: 1,
        })
        .await;
    let mut saw_conflict = false;
    for _ in 0..5 {
        if let Ok(Event::DiagnosticRaised { diagnostic }) = events.recv().await {
            if diagnostic.get("settings_update").and_then(|v| v.as_str()) == Some("failed") {
                saw_conflict = true;
                break;
            }
        }
    }
    assert!(saw_conflict, "expected settings conflict diagnostic");

    let loaded = store.load_or_default().unwrap();
    assert_eq!(loaded.imported_projects.len(), 1);
    assert!(loaded.imported_projects[0].path.contains("proj1"));
    assert!(!loaded
        .imported_projects
        .iter()
        .any(|p| p.path.contains("proj2")));
}

#[tokio::test]
async fn removed_ids_evict_results_by_id() {
    struct RemoveModule {
        manifest: ModuleManifest,
    }

    #[async_trait]
    impl LumaModule for RemoveModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }

        async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
            ModuleState::Ready
        }

        async fn search(&self, _query: Query, sink: SearchSink, cancel: CancellationToken) {
            if cancel.is_cancelled() {
                return;
            }
            let item = SearchItemDto {
                id: "ephemeral-1".into(),
                module_id: self.manifest.id.as_str().to_string(),
                title: "Ephemeral".into(),
                subtitle: None,
                kind: "fake".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![item],
                    removed_ids: vec![],
                })
                .await;
            if cancel.is_cancelled() {
                return;
            }
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 2,
                    upserts: vec![],
                    removed_ids: vec!["ephemeral-1".into()],
                })
                .await;
        }

        async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
            vec![ActionDescriptor {
                id: ActionId::new("open"),
                label: "Open".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }]
        }

        async fn perform(
            &self,
            _action: ActionRequest,
            _cancel: CancellationToken,
        ) -> ActionOutcome {
            ActionOutcome::Success {
                message: Some("ok".into()),
            }
        }

        async fn teardown(&self) {}
    }

    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(RemoveModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.remove"),
                display_name: "Remove".into(),
                triggers: vec!["rm".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
    engine
        .handle_command(Command::Search {
            request_id: "r-rm".into(),
            query: "/rm x".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}

    engine
        .handle_command(Command::ListActions {
            result_id: "ephemeral-1".into(),
        })
        .await;
    let actions = loop {
        if let Ok(Event::ActionsAvailable { actions, .. }) = events.recv().await {
            break actions;
        }
    };
    assert!(
        actions.is_empty(),
        "removed id must not resolve actions: {actions:?}"
    );

    engine
        .handle_command(Command::ExecuteAction {
            operation_id: "op-rm".into(),
            result_id: "ephemeral-1".into(),
            action_id: "open".into(),
            confirmation: false,
        })
        .await;
    let outcome = loop {
        if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
            break outcome;
        }
    };
    assert!(
        matches!(outcome, luma_protocol::ActionOutcomeDto::Failed { .. }),
        "removed id must not execute: {outcome:?}"
    );
}

#[tokio::test]
async fn refresh_wordbook_review_stats_emits_event() {
    use crate::ports::WordbookRepository;
    use crate::{MemoryWordbookRepository, WordContentInput};
    use std::sync::Arc;
    let store = Arc::new(MemoryWordbookRepository::new());
    store
        .upsert_content(&WordContentInput {
            term: "alpha".into(),
            phonetic: "".into(),
            meaning: "a".into(),
            example: "".into(),
            category: "".into(),
        })
        .unwrap();
    let engine = Engine::with_options(
        fake_registry(),
        EngineOptions {
            settings: None,
            wordbook: Some(store),
            command_recipes: None,
        },
    );
    let mut events = engine.subscribe();
    engine
        .handle_command(Command::RefreshWordbookReviewStats)
        .await;
    let event = loop {
        if let Ok(Event::WordbookReviewStatsUpdated { stats }) = events.recv().await {
            break stats;
        }
    };
    assert!(event.goal >= 1);
}

#[tokio::test]
async fn load_wordbook_review_registers_gradeable_result() {
    use crate::ports::WordbookRepository;
    use crate::{MemoryWordbookRepository, WordContentInput};
    use std::sync::Arc;

    let store = Arc::new(MemoryWordbookRepository::new());
    store
        .upsert_content(&WordContentInput {
            term: "alpha".into(),
            phonetic: "".into(),
            meaning: "a".into(),
            example: "".into(),
            category: "".into(),
        })
        .unwrap();
    let word_id = store.get_by_term("alpha").unwrap().unwrap().id;

    let mut registry = fake_registry();
    registry
        .register(Arc::new(FakeModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.wordbook"),
                display_name: "Wordbook".into(),
                triggers: vec!["wb".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            wait_for_cancel: false,
        }))
        .unwrap();
    let engine = Engine::with_options(
        registry,
        EngineOptions {
            settings: None,
            wordbook: Some(store),
            command_recipes: None,
        },
    );
    let mut events = engine.subscribe();

    engine
        .handle_command(Command::LoadWordbookReview {
            queue: "new".into(),
        })
        .await;
    let loaded = loop {
        if let Ok(Event::WordbookReviewLoaded { words, .. }) = events.recv().await {
            break words;
        }
    };
    assert_eq!(loaded[0].id, word_id);

    engine
        .handle_command(Command::ExecuteAction {
            operation_id: "op-review".into(),
            result_id: format!("wb:{word_id}"),
            action_id: "open".into(),
            confirmation: false,
        })
        .await;
    let outcome = loop {
        if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
            break outcome;
        }
    };
    assert!(matches!(
        outcome,
        luma_protocol::ActionOutcomeDto::Success { .. }
    ));
}

#[tokio::test]
async fn broadcast_emit_never_blocks_after_256_events() {
    use crate::port::EnginePort;
    let engine = Engine::new(fake_registry());
    let mut rx = engine.subscribe();
    // Flood without a consumer draining first — producer must not hang.
    for i in 0..320 {
        engine
            .emit(Event::DiagnosticRaised {
                diagnostic: serde_json::json!({ "n": i }),
            })
            .await
            .unwrap();
    }
    // Subscriber may lag; must still be able to recv something or Lagged.
    let mut saw = 0usize;
    for _ in 0..400 {
        match rx.try_recv() {
            Ok(_) => saw += 1,
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                saw += n as usize;
            }
            Err(broadcast::error::TryRecvError::Empty) => break,
            Err(broadcast::error::TryRecvError::Closed) => break,
        }
    }
    assert!(saw > 0, "subscriber should observe flood or lag");
    // Engine still accepts commands after flood.
    engine.start_session().await;
    assert!(matches!(
        rx.recv().await,
        Ok(Event::ModuleStateChanged { .. }) | Ok(Event::SessionReady { .. })
    ));
}

#[tokio::test]
async fn disable_module_purges_results_and_rejects_actions() {
    let engine = Arc::new(Engine::new(fake_registry()));
    let mut events = engine.subscribe();
    engine.start_session().await;
    while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
    engine
        .handle_command(Command::Search {
            request_id: "r1".into(),
            query: "hello".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}

    engine
        .handle_command(Command::SetModuleEnabled {
            module_id: "luma.fake".into(),
            enabled: false,
        })
        .await;
    // Drain module-disabled and results purge events.
    let mut saw_removed = false;
    for _ in 0..20 {
        match tokio::time::timeout(std::time::Duration::from_millis(50), events.recv()).await {
            Ok(Ok(Event::ResultsChunk { removed_ids, .. })) if !removed_ids.is_empty() => {
                saw_removed = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_removed,
        "disable must emit removed_ids for cached results"
    );

    engine
        .handle_command(Command::ListActions {
            result_id: "fake-1".into(),
        })
        .await;
    let actions = loop {
        if let Ok(Event::ActionsAvailable { actions, .. }) = events.recv().await {
            break actions;
        }
    };
    assert!(actions.is_empty(), "disabled module must not list actions");

    engine
        .handle_command(Command::ExecuteAction {
            operation_id: "op-dis".into(),
            result_id: "fake-1".into(),
            action_id: "open".into(),
            confirmation: false,
        })
        .await;
    let outcome = loop {
        if let Ok(Event::ActionFinished { outcome, .. }) = events.recv().await {
            break outcome;
        }
    };
    assert!(
        matches!(outcome, luma_protocol::ActionOutcomeDto::Failed { .. }),
        "disabled module must not execute: {outcome:?}"
    );
}

struct SlowSearchModule {
    manifest: ModuleManifest,
}

#[async_trait]
impl LumaModule for SlowSearchModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, _query: Query, _sink: SearchSink, _cancel: CancellationToken) {
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        Vec::new()
    }

    async fn perform(&self, _action: ActionRequest, _cancel: CancellationToken) -> ActionOutcome {
        ActionOutcome::Success { message: None }
    }

    async fn teardown(&self) {}
}

#[tokio::test]
async fn search_completion_bound_aborts_slow_module() {
    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(SlowSearchModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.slow"),
                display_name: "Slow".into(),
                triggers: vec!["slow".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
        }))
        .expect("register slow");
    let engine = Engine::new(registry);
    let mut events = engine.subscribe();
    let started = std::time::Instant::now();
    engine
        .handle_command(Command::Search {
            request_id: "slow-1".into(),
            query: "slow hello".into(),
        })
        .await;
    let finished = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(Event::SearchFinished { request_id, .. }) = events.recv().await {
                if request_id == "slow-1" {
                    break;
                }
            }
        }
    })
    .await;
    assert!(finished.is_ok(), "slow search must emit SearchFinished");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "slow module search must not block past completion bound"
    );
}

struct BulkSearchModule {
    manifest: ModuleManifest,
    count: usize,
}

#[async_trait]
impl LumaModule for BulkSearchModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, _query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let upserts: Vec<_> = (0..self.count)
            .map(|i| SearchItemDto {
                id: format!("bulk-{i}"),
                module_id: self.manifest.id.as_str().to_string(),
                title: format!("Item {i}"),
                subtitle: None,
                kind: "bulk".into(),
                score: i as f64,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
                ..Default::default()
            })
            .collect();
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        Vec::new()
    }

    async fn perform(&self, _action: ActionRequest, _cancel: CancellationToken) -> ActionOutcome {
        ActionOutcome::Success { message: None }
    }

    async fn teardown(&self) {}
}

#[tokio::test]
async fn engine_evicts_oldest_results_beyond_cap() {
    use super::results::MAX_ENGINE_RESULTS;

    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(BulkSearchModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.bulk"),
                display_name: "Bulk".into(),
                triggers: vec!["bulk".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            count: MAX_ENGINE_RESULTS + 20,
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
    engine
        .handle_command(Command::Search {
            request_id: "bulk-1".into(),
            query: "/bulk x".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}

    engine.handle_command(Command::GetSnapshot).await;
    let (cached, has_bulk_0) = loop {
        if let Ok(Event::SnapshotLoaded { items, .. }) = events.recv().await {
            break (items.len(), items.iter().any(|item| item.id == "bulk-0"));
        }
    };
    assert_eq!(cached, MAX_ENGINE_RESULTS);
    assert!(!has_bulk_0);
}

#[tokio::test]
async fn engine_eviction_emits_removed_ids() {
    use super::results::MAX_ENGINE_RESULTS;

    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(BulkSearchModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.bulk"),
                display_name: "Bulk".into(),
                triggers: vec!["bulk".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            count: MAX_ENGINE_RESULTS + 5,
        }))
        .unwrap();
    let engine = Arc::new(Engine::new(registry));
    let mut events = engine.subscribe();
    engine.start_session().await;
    while !matches!(events.recv().await, Ok(Event::SessionReady { .. })) {}
    engine
        .handle_command(Command::Search {
            request_id: "bulk-2".into(),
            query: "/bulk x".into(),
        })
        .await;

    let mut saw_removed = false;
    loop {
        match events.recv().await {
            Ok(Event::ResultsChunk { removed_ids, .. }) => {
                if removed_ids.iter().any(|id| id.starts_with("bulk-")) {
                    saw_removed = true;
                }
            }
            Ok(Event::SearchFinished { .. }) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    assert!(saw_removed, "eviction must notify TUI via removed_ids");
}

#[tokio::test]
async fn preview_body_is_capped() {
    use super::preview::MAX_PREVIEW_BODY_BYTES;

    struct PreviewModule {
        manifest: ModuleManifest,
        body: String,
    }

    #[async_trait]
    impl LumaModule for PreviewModule {
        fn manifest(&self) -> &ModuleManifest {
            &self.manifest
        }

        async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
            ModuleState::Ready
        }

        async fn search(&self, _query: Query, sink: SearchSink, _cancel: CancellationToken) {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "pv-1".into(),
                        module_id: "luma.preview".into(),
                        title: "Preview".into(),
                        subtitle: None,
                        kind: "preview".into(),
                        score: 1.0,
                        primary_action_id: "open".into(),
                        primary_action_label: "Open".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
        }

        async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
            Vec::new()
        }

        async fn perform(
            &self,
            _action: ActionRequest,
            _cancel: CancellationToken,
        ) -> ActionOutcome {
            ActionOutcome::Success { message: None }
        }

        async fn preview(&self, _result: &SearchItem) -> Option<String> {
            Some(self.body.clone())
        }

        async fn teardown(&self) {}
    }

    let huge = "x".repeat(MAX_PREVIEW_BODY_BYTES + 1024);
    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(PreviewModule {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.preview"),
                display_name: "Preview".into(),
                triggers: vec!["pv".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            body: huge,
        }))
        .unwrap();
    let engine = Engine::new(registry);
    let mut events = engine.subscribe();
    engine.start_session().await;
    engine
        .handle_command(Command::Search {
            request_id: "pv-search".into(),
            query: "/pv x".into(),
        })
        .await;
    while !matches!(events.recv().await, Ok(Event::SearchFinished { .. })) {}
    engine
        .handle_command(Command::LoadPreview {
            result_id: "pv-1".into(),
            preview_id: 1,
        })
        .await;
    let body = loop {
        if let Ok(Event::PreviewLoaded { body, .. }) = events.recv().await {
            break body;
        }
    };
    assert!(body.len() <= MAX_PREVIEW_BODY_BYTES + 32);
    assert!(body.contains("[truncated]"));
}
