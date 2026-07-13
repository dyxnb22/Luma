use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, EventKitPort, LumaModule, ModuleManifest, ModuleState,
    RemindersAuth, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct TodoModule {
    manifest: ModuleManifest,
    eventkit: Arc<dyn EventKitPort>,
}

impl TodoModule {
    pub fn with_eventkit(eventkit: Arc<dyn EventKitPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.todo"),
                display_name: "Todo".into(),
                triggers: vec!["t".into(), "todo".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["eventkit".into()],
                workbench: Default::default(),
            },
            eventkit,
        }
    }
}

#[async_trait]
impl LumaModule for TodoModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.eventkit.auth_status().await {
            RemindersAuth::Authorized => ModuleState::Ready,
            _ => ModuleState::Cold,
        }
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let status = self.eventkit.auth_status().await;
        if status != RemindersAuth::Authorized {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "todo:permission".into(),
                        module_id: "luma.todo".into(),
                        title: "Reminders gated / unavailable".into(),
                        subtitle: Some(
                            "Needs EventKit auth via a signed host — stub cannot complete access"
                                .into(),
                        ),
                        kind: "permission".into(),
                        score: 0.0,
                        primary_action_id: "request_permission".into(),
                        primary_action_label: "Request".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let rest_raw = query.rest_raw().to_string();
        let rest = query.rest_normalized();

        if rest.starts_with("add ") || rest.starts_with('+') {
            let title = rest_raw
                .trim_start_matches("add ")
                .trim_start_matches("Add ")
                .trim_start_matches("ADD ")
                .trim_start_matches('+')
                .trim();
            if !title.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("todo:new:{title}"),
                            module_id: "luma.todo".into(),
                            title: format!("Create: {title}"),
                            subtitle: Some("Reminders".into()),
                            kind: "create".into(),
                            score: 90.0,
                            primary_action_id: "create".into(),
                            primary_action_label: "Create".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        match self.eventkit.list_incomplete().await {
            Ok(items) => {
                let needle = rest.to_lowercase();
                let mut upserts = Vec::new();
                for item in items {
                    if cancel.is_cancelled() {
                        return;
                    }
                    if needle.is_empty() || item.title.to_lowercase().contains(&needle) {
                        upserts.push(SearchItemDto {
                            id: format!("todo:{}", item.id),
                            module_id: "luma.todo".into(),
                            title: item.title,
                            subtitle: Some("incomplete".into()),
                            kind: "todo".into(),
                            score: 50.0,
                            primary_action_id: "complete".into(),
                            primary_action_label: "Complete".into(),
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
            Err(_) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "todo:permission".into(),
                            module_id: "luma.todo".into(),
                            title: "Reminders permission required".into(),
                            subtitle: Some("list failed — check Privacy → Reminders".into()),
                            kind: "permission".into(),
                            score: 0.0,
                            primary_action_id: "request_permission".into(),
                            primary_action_label: "Request".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
            }
        }
    }
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str().starts_with("todo:new:") {
            return vec![ActionDescriptor {
                id: ActionId::new("create"),
                label: "Create".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.id.as_str() == "todo:permission" {
            return vec![ActionDescriptor {
                id: ActionId::new("request_permission"),
                label: "Request".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![ActionDescriptor {
            id: ActionId::new("complete"),
            label: "Complete".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "request_permission" => match self.eventkit.request_access().await {
                Ok(RemindersAuth::Authorized) => ActionOutcome::Success {
                    message: Some("reminders authorized".into()),
                },
                Ok(_) => ActionOutcome::Failed {
                    kind: FailureKind::PermissionRequired {
                        capability: "eventkit".into(),
                        guidance:
                            "Grant Reminders in System Settings, or run from a signed app host"
                                .into(),
                    },
                },
                Err(err) => ActionOutcome::Failed {
                    kind: FailureKind::PermissionRequired {
                        capability: "eventkit".into(),
                        guidance: err.to_string(),
                    },
                },
            },
            "create" => {
                let title = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("todo:new:")
                    .unwrap_or(action.result.title.as_str());
                match self.eventkit.create(title).await {
                    Ok(item) => ActionOutcome::Success {
                        message: Some(format!("created {}", item.title)),
                    },
                    Err(_) => ActionOutcome::Failed {
                        kind: FailureKind::PermissionRequired {
                            capability: "eventkit".into(),
                            guidance: "Grant Reminders access".into(),
                        },
                    },
                }
            }
            "complete" => {
                let id = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("todo:")
                    .unwrap_or(action.result.id.as_str());
                match self.eventkit.complete(id).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some("completed".into()),
                    },
                    Err(_) => ActionOutcome::Failed {
                        kind: FailureKind::PermissionRequired {
                            capability: "eventkit".into(),
                            guidance: "Grant Reminders access".into(),
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
    async fn teardown(&self) {}
}
