use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, KeychainPort, LumaModule, ModuleManifest, ModuleState,
    PasteboardPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::clipboard_privacy::ClipboardSuppression;

const DEFAULT_IDLE_LOCK_SECS: u32 = 300;

/// Secrets is last / default-off. Values never enter search results.
/// Unlock / lock / copy always require confirmation (search DTO and `actions()` agree).
pub struct SecretsModule {
    manifest: ModuleManifest,
    unlocked: Arc<AtomicBool>,
    idle_lock_secs: Arc<AtomicU32>,
    last_activity: Arc<Mutex<Instant>>,
    idle_cancel: Mutex<Option<CancellationToken>>,
    idle_handle: Mutex<Option<JoinHandle<()>>>,
    keychain: Arc<dyn KeychainPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    suppression: Arc<ClipboardSuppression>,
}

impl SecretsModule {
    pub fn with_deps(
        keychain: Arc<dyn KeychainPort>,
        pasteboard: Arc<dyn PasteboardPort>,
        suppression: Arc<ClipboardSuppression>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.secrets"),
                display_name: "Secrets".into(),
                triggers: vec!["sec".into(), "secret".into(), "secrets".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["keychain".into()],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("V".into()),
                    suggested_query: Some("sec ".into()),
                    empty_hint: Some(
                        "sec · Enter unlocks vault (confirm) · labels only, never values".into(),
                    ),
                    supports_browse: false,
                },
            },
            unlocked: Arc::new(AtomicBool::new(false)),
            idle_lock_secs: Arc::new(AtomicU32::new(DEFAULT_IDLE_LOCK_SECS)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            idle_cancel: Mutex::new(None),
            idle_handle: Mutex::new(None),
            keychain,
            pasteboard,
            suppression,
        }
    }

    async fn touch_activity(&self) {
        *self.last_activity.lock().await = Instant::now();
    }

    async fn start_idle_poller(&self, parent: CancellationToken) {
        self.stop_idle_poller().await;
        let cancel = parent.child_token();
        let unlocked = self.unlocked.clone();
        let idle_lock_secs = self.idle_lock_secs.clone();
        let last_activity = self.last_activity.clone();
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        if !unlocked.load(Ordering::SeqCst) {
                            continue;
                        }
                        let secs = idle_lock_secs.load(Ordering::Relaxed);
                        if secs == 0 {
                            continue;
                        }
                        let elapsed = last_activity.lock().await.elapsed().as_secs();
                        if elapsed >= u64::from(secs) {
                            unlocked.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
        });
        *self.idle_cancel.lock().await = Some(cancel);
        *self.idle_handle.lock().await = Some(handle);
    }

    async fn stop_idle_poller(&self) {
        if let Some(cancel) = self.idle_cancel.lock().await.take() {
            cancel.cancel();
        }
        if let Some(handle) = self.idle_handle.lock().await.take() {
            let _ = handle.await;
        }
    }
}

#[async_trait]
impl LumaModule for SecretsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if !ctx.cancel.is_cancelled() {
            self.start_idle_poller(ctx.cancel).await;
        }
        match self.keychain.list_labels().await {
            Ok(_) => ModuleState::Ready,
            Err(err) => ModuleState::Failed(err.to_string()),
        }
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let unlocked = self.unlocked.load(Ordering::SeqCst);
        if unlocked {
            self.touch_activity().await;
        }
        let mut upserts = vec![SearchItemDto {
            id: "sec:vault".into(),
            module_id: "luma.secrets".into(),
            title: if unlocked {
                "Secrets vault".into()
            } else {
                "Secrets vault locked".into()
            },
            subtitle: Some("labels only — values never in search".into()),
            kind: "secrets".into(),
            score: 1.0,
            primary_action_id: if unlocked { "lock" } else { "unlock" }.into(),
            primary_action_label: if unlocked { "Lock" } else { "Unlock" }.into(),
            primary_action_risk: ActionRisk::Confirm,
            primary_action_confirmation: true,
            ..Default::default()
        }];

        if unlocked {
            let needle = query
                .normalized
                .split_once(|c: char| c.is_whitespace())
                .map(|(_, r)| r.trim().to_string())
                .unwrap_or_default()
                .to_lowercase();
            match self.keychain.list_labels().await {
                Err(err) => {
                    upserts.push(SearchItemDto {
                        id: "sec:unavailable".into(),
                        module_id: "luma.secrets".into(),
                        title: "Secrets keychain unavailable".into(),
                        subtitle: Some(crate::ux::friendly_store_error(&err.to_string())),
                        kind: "unavailable".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Unavailable".into(),
                        ..Default::default()
                    });
                }
                Ok(labels) if labels.is_empty() => {
                    upserts.push(SearchItemDto {
                        id: "sec:not_configured".into(),
                        module_id: "luma.secrets".into(),
                        title: "No secrets labels yet".into(),
                        subtitle: Some(
                            "Run: luma secrets set <account>  (value from stdin)".into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "seed_config".into(),
                        primary_action_label: "Show command".into(),
                        ui_intent: Some(UiIntent::SeedConfig),
                        ..Default::default()
                    });
                }
                Ok(labels) => {
                    for label in labels {
                        if cancel.is_cancelled() {
                            return;
                        }
                        if needle.is_empty() || label.account.to_lowercase().contains(&needle) {
                            upserts.push(SearchItemDto {
                                id: format!("sec:{}", label.account),
                                module_id: "luma.secrets".into(),
                                title: label.account.clone(),
                                subtitle: Some("label only — copy to pasteboard".into()),
                                kind: "secret".into(),
                                score: 40.0,
                                primary_action_id: "copy".into(),
                                primary_action_label: "Copy".into(),
                                primary_action_risk: ActionRisk::Confirm,
                                primary_action_confirmation: true,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }

        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "sec:vault" {
            let unlocked = self.unlocked.load(Ordering::SeqCst);
            return vec![ActionDescriptor {
                id: ActionId::new(if unlocked { "lock" } else { "unlock" }),
                label: if unlocked { "Lock" } else { "Unlock" }.into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }];
        }
        if result.id.as_str() == "sec:not_configured"
            || result.kind == "not_configured"
            || result.primary_action.id.as_str() == "seed_config"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str() == "sec:unavailable"
            || result.kind == "unavailable"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![ActionDescriptor {
            id: ActionId::new("copy"),
            label: "Copy".into(),
            risk: ActionRisk::Confirm,
            confirmation: true,
        }]
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if action.action.confirmation && !action.confirmation {
            return ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "confirmation required".into(),
                },
            };
        }
        match action.action.id.as_str() {
            "noop" | "seed_config" => ActionOutcome::Success {
                message: Some(
                    "Bootstrap: luma secrets set <account>  (value from stdin; updates Keychain + label sidecar)"
                        .into(),
                ),
            },
            // In-process UX gate only: no Touch ID, Keychain ACL, or OS auth prompt.
            // Secret values are still fetched via KeychainPort on confirmed copy.
            "unlock" => {
                self.unlocked.store(true, Ordering::SeqCst);
                self.touch_activity().await;
                ActionOutcome::Success {
                    message: Some("unlocked".into()),
                }
            }
            "lock" => {
                self.unlocked.store(false, Ordering::SeqCst);
                ActionOutcome::Success {
                    message: Some("locked".into()),
                }
            }
            "copy" => {
                if !self.unlocked.load(Ordering::SeqCst) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "vault locked".into(),
                        },
                    };
                }
                let Some(account) = action.result.id.as_str().strip_prefix("sec:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected sec:<account>".into(),
                        },
                    };
                };
                match self.keychain.copy_password(account).await {
                    Ok(secret) => {
                        self.suppression
                            .suppress(&secret, Duration::from_secs(86_400));
                        match self.pasteboard.write_text(&secret).await {
                            Ok(()) => {
                                self.touch_activity().await;
                                ActionOutcome::Success {
                                    message: Some(
                                        "copied (suppressed from clipboard history)".into(),
                                    ),
                                }
                            }
                            Err(err) => ActionOutcome::Failed {
                                kind: FailureKind::Unavailable {
                                    reason: err.to_string(),
                                    retryable: true,
                                },
                            },
                        }
                    }
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: false,
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

    async fn apply_settings(&self, settings: &luma_application::AppSettings) {
        self.idle_lock_secs
            .store(settings.secrets_idle_lock_secs, Ordering::Relaxed);
    }

    async fn teardown(&self) {
        self.unlocked.store(false, Ordering::SeqCst);
        self.stop_idle_poller().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeKeychain, KeychainError, KeychainPort, PasteboardError, PasteboardPort, SecretLabel,
    };
    use tokio::sync::mpsc;
    use tokio::sync::Mutex as TokioMutex;

    struct MemPb(TokioMutex<Option<String>>);

    #[async_trait]
    impl PasteboardPort for MemPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.0.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            *self.0.lock().await = Some(text.into());
            Ok(())
        }
    }

    fn unlock_request() -> ActionRequest {
        ActionRequest {
            result: SearchItem {
                id: luma_domain::ResultId::new("sec:vault"),
                module_id: ModuleId::new("luma.secrets"),
                title: "Secrets vault locked".into(),
                subtitle: None,
                kind: "secrets".into(),
                score: 1.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("unlock"),
                    label: "Unlock".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: None,
            },
            action: ActionDescriptor {
                id: ActionId::new("unlock"),
                label: "Unlock".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            },
            confirmation: true,
        }
    }

    #[tokio::test]
    async fn search_never_includes_secret_values() {
        let kc = Arc::new(FakeKeychain {
            unlocked: true,
            entries: TokioMutex::new(
                [("api".into(), "super-secret-password-value".into())]
                    .into_iter()
                    .collect(),
            ),
        });
        let m = SecretsModule::with_deps(
            kc,
            Arc::new(MemPb(TokioMutex::new(None))),
            Arc::new(ClipboardSuppression::new()),
        );
        m.unlocked.store(true, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("sec api", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                let blob = serde_json::to_string(&upserts).unwrap();
                assert!(!blob.contains("super-secret-password-value"));
                assert!(upserts.iter().any(|u| u.title == "api"));
                let vault = upserts.iter().find(|u| u.id == "sec:vault").unwrap();
                assert!(vault.primary_action_confirmation);
                assert_eq!(vault.primary_action_risk, ActionRisk::Confirm);
                let api = upserts.iter().find(|u| u.title == "api").unwrap();
                assert!(api.primary_action_confirmation);
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn locked_vault_row_requires_confirmation() {
        let m = SecretsModule::with_deps(
            Arc::new(FakeKeychain {
                unlocked: false,
                entries: TokioMutex::new(Default::default()),
            }),
            Arc::new(MemPb(TokioMutex::new(None))),
            Arc::new(ClipboardSuppression::new()),
        );
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("sec ", 10), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("chunk");
        };
        assert_eq!(upserts.len(), 1);
        assert!(upserts[0].primary_action_confirmation);
        assert_eq!(upserts[0].primary_action_id, "unlock");
        assert_eq!(upserts[0].primary_action_risk, ActionRisk::Confirm);
    }

    #[tokio::test]
    async fn not_configured_when_no_labels() {
        let kc = Arc::new(FakeKeychain {
            unlocked: true,
            entries: TokioMutex::new(Default::default()),
        });
        let m = SecretsModule::with_deps(
            kc,
            Arc::new(MemPb(TokioMutex::new(None))),
            Arc::new(ClipboardSuppression::new()),
        );
        m.unlocked.store(true, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("sec ", 10), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("chunk");
        };
        assert!(upserts.iter().any(|u| u.kind == "not_configured"));
    }

    #[tokio::test]
    async fn list_labels_error_emits_unavailable() {
        struct ErrKc;
        #[async_trait]
        impl KeychainPort for ErrKc {
            async fn list_labels(&self) -> Result<Vec<SecretLabel>, KeychainError> {
                Err(KeychainError::Unavailable("sidecar missing".into()))
            }
            async fn copy_password(&self, _: &str) -> Result<String, KeychainError> {
                Err(KeychainError::Unavailable("locked".into()))
            }
            async fn set_password(&self, _: &str, _: &str) -> Result<(), KeychainError> {
                Ok(())
            }
            async fn delete(&self, _: &str) -> Result<(), KeychainError> {
                Ok(())
            }
        }
        let m = SecretsModule::with_deps(
            Arc::new(ErrKc),
            Arc::new(MemPb(TokioMutex::new(None))),
            Arc::new(ClipboardSuppression::new()),
        );
        m.unlocked.store(true, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::channel(4);
        m.search(Query::parse("sec ", 10), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("chunk");
        };
        assert!(upserts.iter().any(|u| u.kind == "unavailable"));
    }

    #[tokio::test]
    async fn idle_lock_locks_after_timeout() {
        let kc = Arc::new(FakeKeychain {
            unlocked: true,
            entries: TokioMutex::new(Default::default()),
        });
        let pb = Arc::new(MemPb(TokioMutex::new(None)));
        let m = SecretsModule::with_deps(kc, pb, Arc::new(ClipboardSuppression::new()));
        m.idle_lock_secs.store(1, Ordering::Relaxed);
        let cancel = CancellationToken::new();
        m.start_idle_poller(cancel.clone()).await;
        let outcome = m.perform(unlock_request(), CancellationToken::new()).await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert!(m.unlocked.load(Ordering::SeqCst));
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(
            !m.unlocked.load(Ordering::SeqCst),
            "vault should auto-lock after idle timeout"
        );
        m.teardown().await;
    }

    #[tokio::test]
    async fn copy_requires_unlock() {
        let kc = Arc::new(FakeKeychain {
            unlocked: true,
            entries: TokioMutex::new([("api".into(), "tok".into())].into_iter().collect()),
        });
        let pb = Arc::new(MemPb(TokioMutex::new(None)));
        let m = SecretsModule::with_deps(kc, pb.clone(), Arc::new(ClipboardSuppression::new()));
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("sec:api"),
                        module_id: ModuleId::new("luma.secrets"),
                        title: "api".into(),
                        subtitle: None,
                        kind: "secret".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("copy"),
                            label: "Copy".into(),
                            risk: ActionRisk::Confirm,
                            confirmation: true,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("copy"),
                        label: "Copy".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert!(pb.0.lock().await.is_none());
    }
}
