use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{MacProcessCatalog, ProcessCatalog, ProcessEntry};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct KillProcessModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn ProcessCatalog>,
    cache: Arc<RwLock<Vec<ProcessEntry>>>,
}

impl KillProcessModule {
    pub fn new() -> Self {
        Self::with_catalog(Arc::new(MacProcessCatalog))
    }

    pub fn with_catalog(catalog: Arc<dyn ProcessCatalog>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.kill-process"),
                display_name: "Kill Process".into(),
                triggers: vec!["kill".into(), "quit".into(), "k".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
            },
            catalog,
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for KillProcessModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for KillProcessModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if ctx.cancel.is_cancelled() {
            return ModuleState::Cold;
        }
        if let Ok(list) = self.catalog.list_gui_ish().await {
            *self.cache.write().await = list;
            ModuleState::Ready
        } else {
            ModuleState::Cold
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let needle = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let mut list = self.cache.read().await.clone();
        if list.is_empty() {
            let row = SearchItemDto {
                id: "kill:warming".into(),
                module_id: "luma.kill-process".into(),
                title: "Refreshing process list…".into(),
                subtitle: Some("warming".into()),
                kind: "warming".into(),
                score: 0.0,
                primary_action_id: "refresh".into(),
                primary_action_label: "Refresh".into(),
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![row],
                    removed_ids: vec![],
                })
                .await;
            if let Ok(fresh) = self.catalog.list_gui_ish().await {
                *self.cache.write().await = fresh;
                list = self.cache.read().await.clone();
            } else {
                return;
            }
        }
        let mut upserts = Vec::new();
        for p in list {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty() || p.name.to_lowercase().contains(&needle) {
                upserts.push(SearchItemDto {
                    id: format!("proc:{}", p.pid),
                    module_id: "luma.kill-process".into(),
                    title: p.name,
                    subtitle: Some(format!("pid {}", p.pid)),
                    kind: "process".into(),
                    score: 50.0,
                    primary_action_id: "quit".into(),
                    primary_action_label: "Quit".into(),
                });
            }
            if upserts.len() >= query.limit {
                break;
            }
        }
        if !upserts.is_empty() {
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

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![
            ActionDescriptor {
                id: ActionId::new("quit"),
                label: "Quit".into(),
                risk: ActionRisk::Confirm,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("force"),
                label: "Force Kill".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        ]
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if action.action.id.as_str() == "refresh" {
            return match self.catalog.list_gui_ish().await {
                Ok(list) => {
                    *self.cache.write().await = list;
                    ActionOutcome::Success {
                        message: Some("refreshed".into()),
                    }
                }
                Err(err) => ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                },
            };
        }
        if action.action.confirmation && !action.confirmation {
            return ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "confirmation required".into(),
                },
            };
        }
        let Some(pid) = action
            .result
            .id
            .as_str()
            .strip_prefix("proc:")
            .and_then(|s| s.parse::<u32>().ok())
        else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "result_id".into(),
                    message: "expected proc:<pid>".into(),
                },
            };
        };
        let force = action.action.id.as_str() == "force";
        match self.catalog.quit(pid, force).await {
            Ok(()) => ActionOutcome::Success {
                message: Some(format!("{} {pid}", if force { "killed" } else { "quit" })),
            },
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: true,
                },
            },
        }
    }

    async fn teardown(&self) {
        self.cache.write().await.clear();
    }
}
