//! `/run` local-runtime console. It is a view over listening processes, not a supervisor.

use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, AppSettings, ImportedProject, LumaModule, ModuleManifest,
    ModuleState, PasteboardPort, RuntimeError, RuntimeListener, RuntimePort, SearchMode,
    SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.runtime";
const MAX_RUNTIME_RESULTS: usize = 100;

pub struct RuntimeModule {
    manifest: ModuleManifest,
    runtime: Arc<dyn RuntimePort>,
    pasteboard: Arc<dyn PasteboardPort>,
    projects: Arc<RwLock<Vec<ImportedProject>>>,
    cache: Arc<RwLock<Vec<RuntimeListener>>>,
}

impl RuntimeModule {
    pub fn with_deps(
        projects: Vec<ImportedProject>,
        runtime: Arc<dyn RuntimePort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Runtime".into(),
                triggers: vec!["run".into(), "ports".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("R".into()),
                    suggested_query: Some("/run ".into()),
                    empty_hint: Some(
                        "/run · local TCP listeners · terminate needs confirmation".into(),
                    ),
                    supports_browse: false,
                },
            },
            runtime,
            pasteboard,
            projects: Arc::new(RwLock::new(projects)),
            cache: Arc::new(RwLock::new(vec![])),
        }
    }

    async fn associated_project(&self, listener: &RuntimeListener) -> Option<ImportedProject> {
        let cwd = listener.cwd.as_ref()?;
        self.projects
            .read()
            .await
            .iter()
            .find(|project| cwd.starts_with(Path::new(&project.path)))
            .cloned()
    }
    fn item_id(listener: &RuntimeListener) -> String {
        format!(
            "run:{}:{}:{}",
            listener.pid, listener.port, listener.address
        )
    }
    async fn listener_row(&self, listener: &RuntimeListener) -> SearchItemDto {
        let associated = self.associated_project(listener).await;
        let title = format!(
            "{} · {}:{}",
            listener.process_name, listener.address, listener.port
        );
        let mut details = vec![format!("PID {}", listener.pid)];
        if let Some(user) = &listener.user {
            details.push(user.clone());
        }
        if let Some(cwd) = &listener.cwd {
            details.push(cwd.display().to_string());
        }
        if let Some(project) = &associated {
            details.push(format!(
                "project {}",
                project.name.clone().unwrap_or_else(|| "project".into())
            ));
        }
        let payload = serde_json::json!({
            "pid": listener.pid, "port": listener.port, "address": listener.address,
            "process": listener.process_name, "project_path": associated.as_ref().map(|project| project.path.clone()),
            "surface_query": associated.as_ref().map(|project| format!("/proj show {}", project.path)),
        });
        SearchItemDto {
            id: Self::item_id(listener),
            module_id: MODULE_ID.into(),
            title,
            subtitle: Some(details.join(" · ")),
            kind: "runtime_listener".into(),
            score: 80.0,
            primary_action_id: if associated.is_some() {
                "open_project"
            } else {
                "copy_port"
            }
            .into(),
            primary_action_label: if associated.is_some() {
                "Open project"
            } else {
                "Copy port"
            }
            .into(),
            ui_intent: Some(UiIntent::OpenSurface).filter(|_| associated.is_some()),
            action_payload: Some(payload),
            ..Default::default()
        }
    }
    async fn copy(&self, text: String) -> ActionOutcome {
        self.pasteboard
            .write_text(&text)
            .await
            .map(|_| ActionOutcome::Success {
                message: Some("copied".into()),
            })
            .unwrap_or_else(|error| ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: error.to_string(),
                    retryable: true,
                },
            })
    }
}

#[async_trait]
impl LumaModule for RuntimeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let filter = query.rest_normalized();
        let listeners = match self.runtime.list_tcp_listeners().await {
            Ok(listeners) => listeners,
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "run:unavailable".into(),
                            module_id: MODULE_ID.into(),
                            title: "Runtime listeners unavailable".into(),
                            subtitle: Some(error.to_string()),
                            kind: "unavailable".into(),
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
        if cancel.is_cancelled() {
            return;
        }
        *self.cache.write().await = listeners.clone();
        let mut rows = Vec::new();
        for listener in listeners {
            if filter.is_empty()
                || format!(
                    "{} {} {} {}",
                    listener.port,
                    listener.address,
                    listener.process_name,
                    listener
                        .cwd
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default()
                )
                .to_lowercase()
                .contains(&filter)
            {
                rows.push(self.listener_row(&listener).await);
            }
        }
        rows.truncate(query.limit.min(MAX_RUNTIME_RESULTS));
        if rows.is_empty() {
            rows.push(SearchItemDto {
                id: if filter.is_empty() {
                    "run:empty".into()
                } else {
                    "run:no-match".into()
                },
                module_id: MODULE_ID.into(),
                title: if filter.is_empty() {
                    "No local TCP listeners".into()
                } else {
                    "No matching TCP listeners".into()
                },
                subtitle: Some(if filter.is_empty() {
                    "Refresh after starting a local service".into()
                } else {
                    format!("No listener is associated with `{filter}`")
                }),
                kind: "status".into(),
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: rows,
                removed_ids: vec![],
            })
            .await;
    }
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind != "runtime_listener" {
            return vec![descriptor("noop", "OK", ActionRisk::Safe)];
        }
        vec![
            descriptor("refresh", "Refresh listeners", ActionRisk::Safe),
            descriptor("copy_port", "Copy port", ActionRisk::Safe),
            descriptor("copy_pid", "Copy PID", ActionRisk::Safe),
            descriptor("copy_address", "Copy address", ActionRisk::Safe),
            descriptor("copy_process", "Copy process", ActionRisk::Safe),
            descriptor(
                "terminate",
                "Terminate gracefully (SIGTERM)",
                ActionRisk::Destructive,
            ),
        ]
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
        let payload = action.result.action_payload.as_ref();
        let value = |key: &str| {
            payload
                .and_then(|payload| payload.get(key))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        match action.action.id.as_str() {
            "refresh" => ActionOutcome::Success {
                message: Some("refreshing listeners".into()),
            },
            "copy_port" => {
                self.copy(
                    payload
                        .and_then(|payload| payload.get("port"))
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                )
                .await
            }
            "copy_pid" => {
                self.copy(
                    payload
                        .and_then(|payload| payload.get("pid"))
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                )
                .await
            }
            "copy_address" => self.copy(value("address").unwrap_or_default()).await,
            "copy_process" => self.copy(value("process").unwrap_or_default()).await,
            "terminate" => {
                let listener = self
                    .cache
                    .read()
                    .await
                    .iter()
                    .find(|listener| Self::item_id(listener) == action.result.id.as_str())
                    .cloned();
                match listener {
                    Some(listener) => match self.runtime.terminate_gracefully(listener).await {
                        Ok(()) => ActionOutcome::Success {
                            message: Some("SIGTERM sent".into()),
                        },
                        Err(error) => ActionOutcome::Failed {
                            kind: runtime_failure(error),
                        },
                    },
                    None => ActionOutcome::Failed {
                        kind: FailureKind::NotFound {
                            entity: "listener changed; refresh first".into(),
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
    async fn apply_settings(&self, settings: &AppSettings) {
        *self.projects.write().await = settings.imported_projects.clone();
    }
    async fn teardown(&self) {}
}

fn descriptor(id: &str, label: &str, risk: ActionRisk) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        confirmation: !matches!(risk, ActionRisk::Safe),
        risk,
    }
}
fn runtime_failure(error: RuntimeError) -> FailureKind {
    match error {
        RuntimeError::PermissionRequired(guidance) => FailureKind::PermissionRequired {
            capability: "process inspection".into(),
            guidance,
        },
        RuntimeError::NotFound => FailureKind::NotFound {
            entity: "listener".into(),
        },
        RuntimeError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        RuntimeError::Timeout => FailureKind::Timeout {
            operation: "runtime".into(),
        },
        RuntimeError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePasteboard, FakeRuntimePort};
    use std::path::PathBuf;
    use tokio::sync::mpsc;
    #[tokio::test]
    async fn runtime_lists_and_uses_project_surface() {
        let listener = RuntimeListener {
            port: 3000,
            address: "127.0.0.1".into(),
            pid: 42,
            process_name: "node".into(),
            user: Some("me".into()),
            cwd: Some(PathBuf::from("/work/app")),
            identity: "42".into(),
        };
        let module = RuntimeModule::with_deps(
            vec![ImportedProject {
                path: "/work/app".into(),
                name: Some("App".into()),
            }],
            FakeRuntimePort::new(vec![listener]),
            Arc::new(FakePasteboard::new()),
        );
        let (tx, mut rx) = mpsc::channel(2);
        module
            .search(Query::parse("run ", 20), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!()
        };
        assert_eq!(upserts[0].ui_intent, Some(UiIntent::OpenSurface));
    }

    #[tokio::test]
    async fn terminate_requires_confirmation_before_calling_runtime_port() {
        let listener = RuntimeListener {
            port: 3000,
            address: "127.0.0.1".into(),
            pid: 42,
            process_name: "node".into(),
            user: Some("me".into()),
            cwd: None,
            identity: "42".into(),
        };
        let runtime = FakeRuntimePort::new(vec![listener.clone()]);
        let module =
            RuntimeModule::with_deps(vec![], runtime.clone(), Arc::new(FakePasteboard::new()));
        *module.cache.write().await = vec![listener.clone()];
        let result = module.listener_row(&listener).await.into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result,
                    action: descriptor(
                        "terminate",
                        "Terminate gracefully (SIGTERM)",
                        ActionRisk::Destructive,
                    ),
                    confirmation: false,
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
        assert!(runtime.terminated.lock().await.is_empty());
    }
}
