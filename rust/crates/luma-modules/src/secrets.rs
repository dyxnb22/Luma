use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, KeychainPort, LumaModule, ModuleManifest, ModuleState,
    PasteboardPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::clipboard_privacy::ClipboardSuppression;

/// Secrets is last / default-off. Values never enter search results.
/// Unlock / lock / copy always require confirmation (search DTO and `actions()` agree).
pub struct SecretsModule {
    manifest: ModuleManifest,
    unlocked: AtomicBool,
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
                        "sec · unlock vault · labels only (values never in search)".into(),
                    ),
                    supports_browse: false,
                },
            },
            unlocked: AtomicBool::new(false),
            keychain,
            pasteboard,
            suppression,
        }
    }
}

#[async_trait]
impl LumaModule for SecretsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
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
                        subtitle: Some(err.to_string()),
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
                            "NotConfigured — add via Keychain service com.luma.next.secrets (see MODULES.md)".into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Help".into(),
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
        if result.id.as_str() == "sec:unavailable"
            || result.id.as_str() == "sec:not_configured"
            || result.kind == "unavailable"
            || result.kind == "not_configured"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "Help".into(),
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
            "noop" => ActionOutcome::Success {
                message: Some(
                    "Bootstrap: security add-generic-password -s com.luma.next.secrets -a <label> -w <secret> -U"
                        .into(),
                ),
            },
            "unlock" => {
                self.unlocked.store(true, Ordering::SeqCst);
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
                        // Lease exact value so clipboard history never stores it, even
                        // when heuristic `looks_secret` would miss a plain token.
                        self.suppression.suppress(&secret, Duration::from_secs(45));
                        match self.pasteboard.write_text(&secret).await {
                            Ok(()) => ActionOutcome::Success {
                                message: Some("copied (suppressed from clipboard history)".into()),
                            },
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
    async fn teardown(&self) {
        self.unlocked.store(false, Ordering::SeqCst);
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
        m.search(Query::parse("sec", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        let Event::ResultsChunk { upserts, .. } = ev else {
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
        m.search(Query::parse("sec", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        let Event::ResultsChunk { upserts, .. } = ev else {
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
        m.search(Query::parse("sec", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        let Event::ResultsChunk { upserts, .. } = ev else {
            panic!("chunk");
        };
        assert!(upserts.iter().any(|u| u.kind == "unavailable"));
    }
}
