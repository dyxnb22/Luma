use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, ProcessCatalogPort,
    ProcessEntry, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
struct ProcessIdentity {
    pid: u32,
    name: String,
    executable: String,
    start_unix: i64,
}

pub struct KillProcessModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn ProcessCatalogPort>,
    cache: Arc<RwLock<Vec<ProcessEntry>>>,
    self_pid: u32,
    parent_pid: Option<u32>,
}

impl KillProcessModule {
    pub fn with_catalog(catalog: Arc<dyn ProcessCatalogPort>) -> Self {
        let self_pid = std::process::id();
        let parent_pid = std::os::unix::process::parent_id();
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.kill-process"),
                display_name: "Kill Process".into(),
                triggers: vec!["kill".into(), "quit".into(), "k".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: Default::default(),
            },
            catalog,
            cache: Arc::new(RwLock::new(Vec::new())),
            self_pid,
            parent_pid: Some(parent_pid),
        }
    }

    fn is_protected(&self, pid: u32) -> bool {
        pid == self.self_pid || self.parent_pid == Some(pid) || pid == 0 || pid == 1
    }

    fn result_id(p: &ProcessEntry) -> String {
        // Stable identity: pid + start time + full executable (URL-safe separators).
        format!(
            "proc:{}:{}:{}",
            p.pid,
            p.start_unix,
            encode_path(&p.executable)
        )
    }

    fn parse_target(result_id: &str) -> Option<ProcessIdentity> {
        let rest = result_id.strip_prefix("proc:")?;
        let mut parts = rest.splitn(3, ':');
        let pid = parts.next()?.parse().ok()?;
        let start_unix = parts.next()?.parse().ok()?;
        let executable = decode_path(parts.next()?);
        if executable.is_empty() {
            return None;
        }
        let name = executable
            .rsplit('/')
            .next()
            .unwrap_or(&executable)
            .to_string();
        Some(ProcessIdentity {
            pid,
            name,
            executable,
            start_unix,
        })
    }

    fn matches_identity(live: &ProcessEntry, expected: &ProcessIdentity) -> bool {
        live.pid == expected.pid
            && live.start_unix == expected.start_unix
            && live.executable == expected.executable
    }
}

fn encode_path(path: &str) -> String {
    path.replace('%', "%25").replace(':', "%3A")
}

fn decode_path(encoded: &str) -> String {
    encoded.replace("%3A", ":").replace("%25", "%")
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
        let needle = query.rest_normalized();
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
                ..Default::default()
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
            if self.is_protected(p.pid) {
                continue;
            }
            let hay = format!("{} {}", p.name, p.executable).to_lowercase();
            if needle.is_empty() || hay.contains(&needle) {
                upserts.push(SearchItemDto {
                    id: Self::result_id(&p),
                    module_id: "luma.kill-process".into(),
                    title: p.name.clone(),
                    subtitle: Some(format!("pid {} · {}", p.pid, p.executable)),
                    kind: "process".into(),
                    score: 50.0,
                    primary_action_id: "terminate".into(),
                    primary_action_label: "Terminate".into(),
                    primary_action_risk: ActionRisk::Confirm,
                    primary_action_confirmation: true,
                    ..Default::default()
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

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "kill:warming" {
            return vec![ActionDescriptor {
                id: ActionId::new("refresh"),
                label: "Refresh".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![
            ActionDescriptor {
                id: ActionId::new("terminate"),
                label: "Terminate (SIGTERM)".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            },
            ActionDescriptor {
                id: ActionId::new("force"),
                label: "Force Kill (SIGKILL)".into(),
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
        let Some(expected) = Self::parse_target(action.result.id.as_str()) else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "result_id".into(),
                    message: "expected proc:<pid>:<start_unix>:<executable>".into(),
                },
            };
        };
        if self.is_protected(expected.pid) {
            return ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: "refusing to signal self, parent, or system pid".into(),
                },
            };
        }
        // Re-verify full identity after confirmation delay: PID reuse must not kill a different process.
        let live = match self.catalog.list_gui_ish().await {
            Ok(list) => list,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                };
            }
        };
        let Some(current) = live.iter().find(|p| p.pid == expected.pid) else {
            return ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("pid {}", expected.pid),
                },
            };
        };
        if !Self::matches_identity(current, &expected) {
            return ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied {
                    reason: format!(
                        "pid {} identity changed (expected `{}` @{}, now `{}` @{})",
                        expected.pid,
                        expected.executable,
                        expected.start_unix,
                        current.executable,
                        current.start_unix
                    ),
                },
            };
        }
        let force = action.action.id.as_str() == "force";
        let pid = expected.pid;
        let label = expected.name;
        // Cancel boundary: once signal is issued, the effect is committed.
        match await_unless_cancelled(&cancel, self.catalog.quit(pid, force)).await {
            None => ActionOutcome::Cancelled,
            Some(Ok(())) => ActionOutcome::Success {
                message: Some(format!(
                    "{} {pid} ({label})",
                    if force { "killed" } else { "terminated" }
                )),
            },
            Some(Err(err)) => ActionOutcome::Failed {
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
