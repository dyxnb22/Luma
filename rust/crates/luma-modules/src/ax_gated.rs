use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use tokio_util::sync::CancellationToken;

macro_rules! ax_denied_module {
    ($name:ident, $id:expr, $display:expr, $triggers:expr) => {
        pub struct $name {
            manifest: ModuleManifest,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    manifest: ModuleManifest {
                        id: ModuleId::new($id),
                        display_name: $display.into(),
                        triggers: $triggers.into_iter().map(str::to_string).collect(),
                        default_enabled: false,
                        search_mode: SearchMode::TargetedOnly,
                        required_capabilities: vec!["accessibility".into()],
                    },
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait]
        impl LumaModule for $name {
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
                let _ = query;
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("{}:permission", $id),
                            module_id: $id.into(),
                            title: "Accessibility permission required".into(),
                            subtitle: Some("Enable AX in System Settings".into()),
                            kind: "permission".into(),
                            score: 0.0,
                            primary_action_id: "request".into(),
                            primary_action_label: "Request".into(),
                        }],
                        removed_ids: vec![],
                    })
                    .await;
            }
            async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
                vec![ActionDescriptor {
                    id: ActionId::new("request"),
                    label: "Request".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                }]
            }
            async fn perform(
                &self,
                _action: ActionRequest,
                cancel: CancellationToken,
            ) -> ActionOutcome {
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                ActionOutcome::Failed {
                    kind: FailureKind::PermissionRequired {
                        capability: "accessibility".into(),
                        guidance: "Grant Accessibility, then retry".into(),
                    },
                }
            }
            async fn teardown(&self) {}
        }
    };
}

ax_denied_module!(
    WindowLayoutsModule,
    "luma.window-layouts",
    "Window Layouts",
    ["win", "wl", "layout"]
);
ax_denied_module!(
    MenuItemsModule,
    "luma.menu-items",
    "Menu Bar Search",
    ["mb", "menu"]
);

pub struct BrowserTabsModule {
    manifest: ModuleManifest,
}

impl BrowserTabsModule {
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.browser-tabs"),
                display_name: "Browser Tabs".into(),
                triggers: vec!["tab".into(), "tabs".into()],
                default_enabled: false,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["automation".into()],
            },
        }
    }
}

impl Default for BrowserTabsModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for BrowserTabsModule {
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
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: "tab:permission".into(),
                    module_id: "luma.browser-tabs".into(),
                    title: "Automation permission required".into(),
                    subtitle: Some("Grant Apple Events for browsers".into()),
                    kind: "permission".into(),
                    score: 0.0,
                    primary_action_id: "request".into(),
                    primary_action_label: "Request".into(),
                }],
                removed_ids: vec![],
            })
            .await;
    }
    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("request"),
            label: "Request".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }
    async fn perform(&self, _action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        ActionOutcome::Failed {
            kind: FailureKind::PermissionRequired {
                capability: "automation".into(),
                guidance: "Grant Automation for Safari/Chrome, then retry".into(),
            },
        }
    }
    async fn teardown(&self) {}
}
