//! Mihomo controller-first proxy module for local macOS use.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, NetworkProbePort,
    NetworkProbeState, PasteboardPort, ProfileSource, ProfileStorePort, ProfileSummary,
    ProxyCorePort, ProxyMode, ProxyStatus, SearchMode, SearchSink, SystemProxyPort,
    UnavailableNetworkProbe, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

mod errors;
mod profiles;
mod redact;
mod search;

pub(crate) const MODULE_ID: &str = "luma.proxy";

pub struct ProxyModule {
    manifest: ModuleManifest,
    core: Arc<dyn ProxyCorePort>,
    system_proxy: Arc<dyn SystemProxyPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    profiles: Option<Arc<dyn ProfileStorePort>>,
    network_probe: Arc<dyn NetworkProbePort>,
    last_status: RwLock<Option<ProxyStatus>>,
    group_keys: RwLock<HashMap<String, String>>,
    selection_keys: RwLock<HashMap<String, (String, String)>>,
    import_keys: RwLock<HashMap<String, ImportIntent>>,
}

#[derive(Clone)]
pub(super) enum ImportIntent {
    Subscription(String),
    Local(PathBuf),
}

impl ProxyModule {
    pub fn with_deps(
        core: Arc<dyn ProxyCorePort>,
        system_proxy: Arc<dyn SystemProxyPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Proxy".into(),
                triggers: vec!["proxy".into(), "px".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("X".into()),
                    suggested_query: Some("/proxy ".into()),
                    empty_hint: Some("/proxy · inspect Mihomo and system proxy".into()),
                    supports_browse: true,
                    commands: vec![
                        crate::ux::command_spec(
                            "/proxy",
                            "Show Mihomo status and proxy-group overview",
                            "/proxy ",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy status",
                            "Show Mihomo, controller, profiles, and macOS proxy status",
                            "/proxy status",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy check",
                            "Run on-demand local route, DNS, listener, and controller checks",
                            "/proxy check",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy mode",
                            "Choose between Rule and Global routing",
                            "/proxy mode",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy group <name>",
                            "Drill into matching proxy groups and nodes",
                            "/proxy group ",
                            Some("/proxy group Proxy"),
                        ),
                        crate::ux::command_spec(
                            "/proxy global",
                            "Prepare a confirmed switch to Global mode",
                            "/proxy global",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy rule",
                            "Prepare a confirmed switch to Rule mode",
                            "/proxy rule",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy profile [filter]",
                            "List Luma-owned and read-only external profiles",
                            "/proxy profile ",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy profile refresh",
                            "List refreshable Luma-owned subscriptions",
                            "/proxy profile refresh",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy import <https-url|yaml-path>",
                            "Validate and prepare a Luma-owned profile import",
                            "/proxy import ",
                            Some("/proxy import /tmp/profile.yaml"),
                        ),
                        crate::ux::command_spec(
                            "/proxy sync",
                            "Compile proxy.yaml into a draft Profile",
                            "/proxy sync",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy apply",
                            "Compile and apply proxy.yaml to the running Mihomo",
                            "/proxy apply",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/proxy refresh",
                            "Refresh proxy providers in the running Mihomo",
                            "/proxy refresh",
                            None,
                        ),
                    ],
                },
            },
            core,
            system_proxy,
            pasteboard,
            profiles: None,
            network_probe: Arc::new(UnavailableNetworkProbe),
            last_status: RwLock::new(None),
            group_keys: RwLock::new(HashMap::new()),
            selection_keys: RwLock::new(HashMap::new()),
            import_keys: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_profile_store(mut self, profiles: Arc<dyn ProfileStorePort>) -> Self {
        self.profiles = Some(profiles);
        self
    }

    pub fn with_network_probe(mut self, network_probe: Arc<dyn NetworkProbePort>) -> Self {
        self.network_probe = network_probe;
        self
    }

    async fn set_global_mode(&self) -> Result<(), luma_application::ProxyCoreError> {
        let groups = self.core.list_proxy_groups().await?;
        let global = groups
            .iter()
            .find(|group| group.name.eq_ignore_ascii_case("GLOBAL"));
        let route_via_proxy = global.and_then(|group| {
            let direct = group.selected.as_deref().is_none_or(|selected| {
                selected.eq_ignore_ascii_case("DIRECT") || selected.eq_ignore_ascii_case("REJECT")
            });
            let proxy = group
                .nodes
                .iter()
                .find(|node| node.name.eq_ignore_ascii_case("PROXY"))?;
            direct.then(|| {
                (
                    group.name.clone(),
                    proxy.name.clone(),
                    group.selected.clone(),
                )
            })
        });

        if let Some((group, proxy, _)) = &route_via_proxy {
            self.core.select_proxy(group, proxy).await?;
        }
        if let Err(error) = self.core.set_mode(ProxyMode::Global).await {
            if let Some((group, _, Some(previous))) = route_via_proxy {
                let _ = self.core.select_proxy(&group, &previous).await;
            }
            return Err(error);
        }
        Ok(())
    }
}

#[async_trait]
impl LumaModule for ProxyModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.core.get_status().await {
            Ok(status) => {
                *self.last_status.write().await = Some(status);
                ModuleState::Ready
            }
            Err(error) => ModuleState::Failed(error.to_string()),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        self.search_ready(&query, &sink, &cancel).await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.kind.as_str() {
            "unavailable" | "permission_required" => vec![result.primary_action.clone()],
            "proxy_node" => {
                let mut actions = vec![result.primary_action.clone()];
                actions.extend(result.secondary_actions.iter().cloned());
                actions
            }
            "proxy_group" => vec![result.primary_action.clone()],
            "proxy_mode" => {
                let global = result.primary_action.id.as_str() == "set_global";
                vec![ActionDescriptor {
                    id: ActionId::new(if global { "set_global" } else { "set_rule" }),
                    label: if global { "Set Global" } else { "Set Rule" }.into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                }]
            }
            "profile" => {
                let mut actions = vec![ActionDescriptor {
                    id: ActionId::new(result.primary_action.id.as_str()),
                    label: result.primary_action.label.clone(),
                    risk: result.primary_action.risk.clone(),
                    confirmation: result.primary_action.confirmation,
                }];
                if result.primary_action.id.as_str() == "use_profile" {
                    actions.extend(result.secondary_actions.iter().filter_map(|action| {
                        matches!(action.id.as_str(), "delete_profile" | "refresh_profile")
                            .then_some(action.clone())
                    }));
                }
                actions
            }
            "profile_import" => vec![ActionDescriptor {
                id: ActionId::new("import_profile"),
                label: "Import".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }],
            "profile_sync" => vec![ActionDescriptor {
                id: ActionId::new("sync_convention_profile"),
                label: "Sync".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }],
            "profile_apply" => vec![ActionDescriptor {
                id: ActionId::new("apply_convention_profile"),
                label: "Apply".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }],
            "provider_refresh" => vec![ActionDescriptor {
                id: ActionId::new("refresh_providers"),
                label: "Refresh Providers".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "profile_import_help" => vec![],
            _ => {
                let mut actions = vec![result.primary_action.clone()];
                for action in &result.secondary_actions {
                    if !actions
                        .iter()
                        .any(|existing| existing.id.as_str() == action.id.as_str())
                    {
                        actions.push(action.clone());
                    }
                }
                actions
            }
        }
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        Some(match result.kind.as_str() {
            "status" => result
                .subtitle
                .clone()
                .unwrap_or_else(|| result.title.clone()),
            "proxy_node" => format!(
                "{}\n{}",
                result.title,
                result.subtitle.clone().unwrap_or_default()
            ),
            _ => result
                .subtitle
                .clone()
                .unwrap_or_else(|| result.title.clone()),
        })
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        use errors::{profile_failure, proxy_failure, system_failure};
        use redact::redact_label;

        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let id = action.action.id.as_str();
        match id {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "open_proxy_overview" | "retry_proxy" => ActionOutcome::OpenSurface {
                query: "/proxy ".into(),
            },
            "open_proxy_status" => ActionOutcome::OpenSurface {
                query: "/proxy status".into(),
            },
            "rerun_proxy_check" => ActionOutcome::OpenSurface {
                query: "/proxy check".into(),
            },
            "open_proxy_modes" => ActionOutcome::OpenSurface {
                query: "/proxy mode".into(),
            },
            "open_proxy_profiles" => ActionOutcome::OpenSurface {
                query: "/proxy profile".into(),
            },
            "open_proxy_group" => {
                let Some(group) = self
                    .group_keys
                    .read()
                    .await
                    .get(action.result.id.as_str())
                    .cloned()
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "proxy group result expired; search again".into(),
                        },
                    };
                };
                if group.chars().any(char::is_control) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "group".into(),
                            message: "proxy group name contains unsupported control characters"
                                .into(),
                        },
                    };
                }
                ActionOutcome::OpenSurface {
                    query: format!("/proxy group {group}"),
                }
            }
            "set_global" => match self.set_global_mode().await {
                Ok(()) => ActionOutcome::Success {
                    message: Some("mode set to Global".into()),
                },
                Err(error) => ActionOutcome::Failed {
                    kind: proxy_failure(error),
                },
            },
            "set_rule" => match self.core.set_mode(ProxyMode::Rule).await {
                Ok(()) => ActionOutcome::Success {
                    message: Some("mode set to Rule".into()),
                },
                Err(error) => ActionOutcome::Failed {
                    kind: proxy_failure(error),
                },
            },
            "refresh" | "refresh_providers" => match self.core.refresh_provider().await {
                Ok(()) => ActionOutcome::Success {
                    message: Some("proxy providers refreshed".into()),
                },
                Err(error) => ActionOutcome::Failed {
                    kind: proxy_failure(error),
                },
            },
            "import_profile" => {
                let Some(intent) = self
                    .import_keys
                    .write()
                    .await
                    .remove(action.result.id.as_str())
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "import request expired; search again".into(),
                        },
                    };
                };
                let Some(store) = &self.profiles else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Profile storage is not configured".into(),
                        },
                    };
                };
                let result = match intent {
                    ImportIntent::Subscription(url) => store.import_subscription(&url, None).await,
                    ImportIntent::Local(path) => store.import_local_file(&path, None).await,
                };
                match result {
                    Ok(result) => ActionOutcome::Success {
                        message: Some(format!(
                            "已导入 Profile：{}；尚未应用到运行中的 Mihomo",
                            redact_label(&result.summary.name)
                        )),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: profile_failure(error),
                    },
                }
            }
            "sync_convention_profile" => {
                let Some(store) = &self.profiles else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Profile storage is not configured".into(),
                        },
                    };
                };
                match store.sync_convention_profile().await {
                    Ok(result) => ActionOutcome::Success {
                        message: Some(format!(
                            "已编译约定 Profile：{}（{} 个节点）；尚未应用到运行中的 Mihomo",
                            redact_label(&result.summary.name),
                            result.summary.node_count
                        )),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: profile_failure(error),
                    },
                }
            }
            "apply_convention_profile" => {
                let Some(store) = &self.profiles else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Profile storage is not configured".into(),
                        },
                    };
                };
                let synced = match store.sync_convention_profile().await {
                    Ok(result) => result,
                    Err(error) => {
                        return ActionOutcome::Failed {
                            kind: profile_failure(error),
                        };
                    }
                };
                match store.use_profile(&synced.summary.id).await {
                    Ok(result) if result.runtime_applied => ActionOutcome::Success {
                        message: Some(format!(
                            "已应用 proxy.yaml：{}（{} 个节点）",
                            redact_label(&result.summary.name),
                            result.summary.node_count
                        )),
                    },
                    Ok(_) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: "Profile was saved but the Mihomo runtime did not apply it"
                                .into(),
                            retryable: true,
                        },
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: profile_failure(error),
                    },
                }
            }
            "use_profile" | "delete_profile" | "refresh_profile" => {
                let Some(profile_id) = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("profile_id"))
                    .and_then(Value::as_str)
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "profile_id".into(),
                            message: "missing opaque Profile identifier".into(),
                        },
                    };
                };
                let Some(store) = &self.profiles else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::NotConfigured {
                            remediation: "Profile storage is not configured".into(),
                        },
                    };
                };
                let result = match id {
                    "use_profile" => store
                        .use_profile(profile_id)
                        .await
                        .map(|r| (r, "Profile 已应用")),
                    "refresh_profile" => store
                        .refresh_profile(profile_id)
                        .await
                        .map(|r| (r, "Profile 已刷新；尚未应用到运行中的 Mihomo")),
                    _ => store.delete_profile(profile_id).await.map(|_| {
                        (
                            luma_application::ProfileImportResult {
                                summary: ProfileSummary {
                                    id: profile_id.into(),
                                    name: "Profile".into(),
                                    node_count: 0,
                                    group_count: 0,
                                    rule_count: 0,
                                    metadata_available: true,
                                    updated_at: None,
                                    source: ProfileSource::LumaLocal,
                                    owned_by_luma: true,
                                    current: false,
                                },
                                source_written: false,
                                metadata_updated: true,
                                runtime_applied: false,
                            },
                            "Profile 已删除",
                        )
                    }),
                };
                match result {
                    Ok((result, message)) => ActionOutcome::Success {
                        message: Some(if id == "use_profile" && !result.runtime_applied {
                            "已导入，尚未应用到运行中的 Mihomo".into()
                        } else {
                            message.into()
                        }),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: profile_failure(error),
                    },
                }
            }
            "select_proxy" => {
                let Some((group, proxy)) = self
                    .selection_keys
                    .read()
                    .await
                    .get(action.result.id.as_str())
                    .cloned()
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "proxy result expired; search again".into(),
                        },
                    };
                };
                match self.core.select_proxy(&group, &proxy).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some("proxy selected".into()),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: proxy_failure(error),
                    },
                }
            }
            "test_proxy" => {
                let Some((_group, proxy)) = self
                    .selection_keys
                    .read()
                    .await
                    .get(action.result.id.as_str())
                    .cloned()
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "proxy result expired; search again".into(),
                        },
                    };
                };
                match self.core.test_proxy_delay(&proxy).await {
                    Ok(delay_ms) => ActionOutcome::Success {
                        message: Some(format!("节点延迟：{} ms", delay_ms)),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: proxy_failure(error),
                    },
                }
            }
            "enable_system_proxy" => {
                let status = match self.core.get_status().await {
                    Ok(status) => status,
                    Err(error) => {
                        return ActionOutcome::Failed {
                            kind: proxy_failure(error),
                        }
                    }
                };
                let mut ports = vec![
                    status.ports.http.or(status.ports.mixed),
                    status.ports.socks.or(status.ports.mixed),
                ];
                ports.sort_unstable();
                ports.dedup();
                for port in ports.into_iter().flatten() {
                    let probe = self.network_probe.loopback_listener(port).await;
                    if probe.state == NetworkProbeState::Fail {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: format!(
                                    "Mihomo listener 127.0.0.1:{port} is not accepting connections"
                                ),
                                retryable: true,
                            },
                        };
                    }
                }
                match self
                    .system_proxy
                    .enable(
                        status.ports.http.or(status.ports.mixed),
                        status.ports.http.or(status.ports.mixed),
                        status.ports.socks.or(status.ports.mixed),
                    )
                    .await
                {
                    Ok(_) => ActionOutcome::Success {
                        message: Some("system proxy enabled".into()),
                    },
                    Err(error) => ActionOutcome::Failed {
                        kind: system_failure(error),
                    },
                }
            }
            "disable_system_proxy" => match self.system_proxy.disable().await {
                Ok(_) => ActionOutcome::Success {
                    message: Some("system proxy disabled".into()),
                },
                Err(error) => ActionOutcome::Failed {
                    kind: system_failure(error),
                },
            },
            "copy_proxy_address" => {
                let Some(address) = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("address"))
                    .and_then(Value::as_str)
                else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "address".into(),
                            message: "no local proxy address available".into(),
                        },
                    };
                };
                match await_unless_cancelled(&cancel, self.pasteboard.write_text(address)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("proxy address copied".into()),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
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
        *self.last_status.write().await = None;
        self.group_keys.write().await.clear();
        self.selection_keys.write().await.clear();
        self.import_keys.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::errors::proxy_failure;
    use super::profiles::profile_item;
    use super::redact::redact_label;
    use super::*;
    use luma_application::{
        FakeNetworkProbe, FakePasteboard, FakeProxyCore, FakeSystemProxy, NetworkProbeState,
        NetworkProbeStep, ProfileImportResult, ProfileSource, ProfileStoreError, ProfileStorePort,
        ProxyCoreError, ProxyCoreKind, ProxyGroup, ProxyNode, ProxyPorts, SystemProxySetting,
        SystemProxyStatus,
    };
    use luma_protocol::SearchItemDto;
    use luma_test_support::collect_search_items;
    use std::path::Path;

    fn module() -> (ProxyModule, Arc<FakeProxyCore>, Arc<FakeSystemProxy>) {
        let core = FakeProxyCore::new(
            ProxyStatus {
                running: true,
                core_kind: ProxyCoreKind::Standalone,
                mode: ProxyMode::Rule,
                profile: Some("V2Box AI Split".into()),
                ports: ProxyPorts {
                    http: Some(7899),
                    mixed: Some(7897),
                    socks: Some(7898),
                },
                allow_lan: false,
                tun_enabled: false,
            },
            vec![ProxyGroup {
                name: "AI-VPS".into(),
                selected: Some("V2Box-VPS".into()),
                nodes: vec![ProxyNode {
                    name: "V2Box-VPS".into(),
                    kind: "VLESS".into(),
                    delay_ms: Some(88),
                    selected: true,
                    group: Some("AI-VPS".into()),
                }],
            }],
        );
        let system = FakeSystemProxy::new(SystemProxyStatus {
            service: "Wi-Fi".into(),
            http: SystemProxySetting::default(),
            https: SystemProxySetting::default(),
            socks: SystemProxySetting::default(),
        });
        let module = ProxyModule::with_deps(
            core.clone(),
            system.clone(),
            Arc::new(FakePasteboard::new()),
        );
        (module, core, system)
    }

    #[tokio::test]
    async fn overview_shows_settings_and_node_drilldown_shows_only_nodes() {
        let (module, core, _) = module();
        core.groups.lock().await.push(ProxyGroup {
            name: "COMPATIBLE".into(),
            selected: None,
            nodes: vec![],
        });
        let items = collect_search_items(&module, Query::parse("proxy ", 20)).await;
        assert_eq!(
            items
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>(),
            vec![
                "System Proxy",
                "Proxy Mode",
                "Current Node",
                "Configuration",
                "Connection Check",
                "Runtime",
            ]
        );
        let group = items
            .iter()
            .find(|item| item.title == "Current Node")
            .unwrap()
            .clone();
        assert!(group
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("V2Box-VPS")));
        assert_eq!(group.primary_action.id.as_str(), "open_proxy_group");
        assert!(group.ui_intent.is_none());
        assert!(!items.iter().any(|item| item.title == "COMPATIBLE"));
        assert!(!items.iter().any(|item| item.kind == "proxy_node"));

        let mode = items
            .iter()
            .find(|item| item.title == "Proxy Mode")
            .unwrap()
            .clone();
        assert_eq!(mode.primary_action.id.as_str(), "open_proxy_modes");
        assert_eq!(mode.primary_action.label, "Choose Mode");
        assert!(!mode.primary_action.confirmation);
        let outcome = module
            .perform(
                ActionRequest {
                    result: mode.clone(),
                    action: module.actions(&mode).await.remove(0),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::OpenSurface { query } if query == "/proxy mode"
        ));

        let action = module.actions(&group).await.into_iter().next().unwrap();
        let outcome = module
            .perform(
                ActionRequest {
                    result: group,
                    action,
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::OpenSurface { query } if query == "/proxy group AI-VPS"
        ));

        let items = collect_search_items(&module, Query::parse("/proxy group ai-vps", 20)).await;
        let selected = items.iter().find(|item| item.title == "V2Box-VPS").unwrap();
        assert!(selected
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("selected")));
        assert_eq!(selected.primary_action.id.as_str(), "test_proxy");
        assert!(items.iter().all(|item| item.kind == "proxy_node"));
        assert!(!items.iter().any(|item| item.kind == "proxy_group"));
        assert!(!redact_label("node-123e4567-e89b-12d3-a456-426614174000").contains("123e4567"));
    }

    #[tokio::test]
    async fn mode_page_explicitly_lists_rule_and_global() {
        let (module, core, _) = module();
        let items = collect_search_items(&module, Query::parse("/proxy mode", 20)).await;
        assert_eq!(
            items
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Rule", "Global"]
        );

        let rule = items.iter().find(|item| item.title == "Rule").unwrap();
        assert!(rule
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.starts_with("Selected ·")));
        assert_eq!(rule.primary_action.id.as_str(), "noop");
        assert_eq!(rule.primary_action.label, "Selected");
        assert!(!rule.primary_action.confirmation);

        let global = items.iter().find(|item| item.title == "Global").unwrap();
        assert_eq!(global.primary_action.id.as_str(), "set_global");
        assert_eq!(global.primary_action.label, "Select Global");
        assert!(global.primary_action.confirmation);

        core.status.lock().await.mode = ProxyMode::Global;
        let items = collect_search_items(&module, Query::parse("/proxy mode", 20)).await;
        let rule = items.iter().find(|item| item.title == "Rule").unwrap();
        let global = items.iter().find(|item| item.title == "Global").unwrap();
        assert_eq!(rule.primary_action.id.as_str(), "set_rule");
        assert_eq!(rule.primary_action.label, "Select Rule");
        assert!(rule.primary_action.confirmation);
        assert_eq!(global.primary_action.id.as_str(), "noop");
        assert_eq!(global.primary_action.label, "Selected");
    }

    #[tokio::test]
    async fn incomplete_and_unknown_commands_are_explicit_errors() {
        let (module, _, _) = module();
        for raw in ["/proxy group", "/proxy unknown"] {
            let items = collect_search_items(&module, Query::parse(raw, 20)).await;
            assert_eq!(items.len(), 1, "{raw}");
            assert_eq!(items[0].kind, "command_error", "{raw}");
        }
    }

    #[tokio::test]
    async fn status_and_check_show_https_and_structured_probe_results() {
        let (base, _, _) = module();
        let probe = FakeNetworkProbe::new(vec![NetworkProbeStep {
            name: "Default route".into(),
            state: NetworkProbeState::Pass,
            detail: "available".into(),
            remediation: "connect to a network".into(),
        }]);
        probe.listeners.lock().await.insert(
            7899,
            NetworkProbeStep {
                name: "Local listener 127.0.0.1:7899".into(),
                state: NetworkProbeState::Pass,
                detail: "accepting connections".into(),
                remediation: "start Mihomo".into(),
            },
        );
        let module = base.with_network_probe(probe);
        let status = collect_search_items(&module, Query::parse("proxy status", 20)).await;
        assert!(status
            .iter()
            .any(|item| item.title.starts_with("System HTTP proxy")));
        assert!(status
            .iter()
            .any(|item| item.title.starts_with("System HTTPS proxy")));
        assert!(status
            .iter()
            .any(|item| item.title.starts_with("System SOCKS proxy")));
        let checks = collect_search_items(&module, Query::parse("proxy check", 20)).await;
        assert!(checks
            .iter()
            .any(|item| item.title == "pass · Default route"));
        assert!(checks
            .iter()
            .any(|item| item.title == "pass · Local listener 127.0.0.1:7899"));
    }

    #[tokio::test]
    async fn system_setting_enables_off_and_switches_mismatched_system_proxy() {
        let (module, _, system) = module();
        let off = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:system")
            .unwrap();
        assert_eq!(off.primary_action.id.as_str(), "enable_system_proxy");
        assert_eq!(off.primary_action.label, "Turn On");
        assert_eq!(off.primary_action.risk, ActionRisk::Safe);
        assert!(!off.primary_action.confirmation);

        let mismatched = SystemProxySetting {
            enabled: true,
            server: Some("127.0.0.1".into()),
            port: Some(7897),
        };
        {
            let mut status = system.status.lock().await;
            status.http = mismatched.clone();
            status.https = mismatched.clone();
            status.socks = mismatched;
        }
        let item = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:system")
            .unwrap();
        assert!(item
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("OTHER")));
        assert_eq!(item.primary_action.id.as_str(), "enable_system_proxy");
        assert_eq!(item.primary_action.label, "Use Luma");
        assert_eq!(item.primary_action.risk, ActionRisk::Confirm);
        assert!(item.primary_action.confirmation);
        let actions = module.actions(&item).await;
        assert_eq!(actions[0].id.as_str(), "enable_system_proxy");
        assert!(!actions
            .iter()
            .any(|action| action.id.as_str() == "disable_system_proxy"));
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: actions[0].clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(
            *system.enable_calls.lock().await,
            vec![(Some(7899), Some(7899), Some(7898))]
        );
    }

    #[tokio::test]
    async fn status_requires_all_expected_protocols_before_reporting_on() {
        let (module, _, system) = module();
        {
            let mut status = system.status.lock().await;
            status.http = SystemProxySetting {
                enabled: true,
                server: Some("127.0.0.1".into()),
                port: Some(7899),
            };
        }
        let partial = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:system")
            .unwrap();
        assert!(partial
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("OTHER")));

        {
            let mut status = system.status.lock().await;
            status.https = SystemProxySetting {
                enabled: true,
                server: Some("localhost".into()),
                port: Some(7899),
            };
            status.socks = SystemProxySetting {
                enabled: true,
                server: Some("::1".into()),
                port: Some(7898),
            };
        }
        let on = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:system")
            .unwrap();
        assert!(on
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("ON")));
        assert_eq!(on.primary_action.id.as_str(), "disable_system_proxy");
        assert_eq!(on.primary_action.label, "Turn Off");
        assert!(on.primary_action.confirmation);
    }

    #[tokio::test]
    async fn selecting_node_is_safe_and_calls_core() {
        let (module, core, _) = module();
        core.groups.lock().await[0].nodes[0].selected = false;
        let items = collect_search_items(&module, Query::parse("proxy group AI-VPS", 20)).await;
        let node = items
            .iter()
            .find(|item| item.kind == "proxy_node")
            .unwrap()
            .clone();
        let actions = module.actions(&node).await;
        assert_eq!(actions[0].id.as_str(), "select_proxy");
        assert_eq!(actions[1].id.as_str(), "test_proxy");
        let outcome = module
            .perform(
                ActionRequest {
                    result: node.clone(),
                    action: actions[0].clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(core.selected.lock().await.len(), 1);
        let tested = module
            .perform(
                ActionRequest {
                    result: node,
                    action: actions[1].clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(
            matches!(tested, ActionOutcome::Success { message: Some(ref msg) } if msg.contains("42"))
        );
        assert_eq!(core.delay_tests.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn global_mode_routes_global_selector_through_proxy_group() {
        let (module, core, _) = module();
        core.groups.lock().await.push(ProxyGroup {
            name: "GLOBAL".into(),
            selected: Some("DIRECT".into()),
            nodes: vec![
                ProxyNode {
                    name: "DIRECT".into(),
                    kind: "Direct".into(),
                    delay_ms: None,
                    selected: true,
                    group: Some("GLOBAL".into()),
                },
                ProxyNode {
                    name: "PROXY".into(),
                    kind: "Selector".into(),
                    delay_ms: None,
                    selected: false,
                    group: Some("GLOBAL".into()),
                },
            ],
        });
        let mode = collect_search_items(&module, Query::parse("/proxy mode", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:mode:global")
            .unwrap();
        let action = module.actions(&mode).await.remove(0);
        let outcome = module
            .perform(
                ActionRequest {
                    result: mode,
                    action,
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(
            *core.selected.lock().await,
            vec![("GLOBAL".into(), "PROXY".into())]
        );
        assert_eq!(*core.mode_changes.lock().await, vec![ProxyMode::Global]);
    }

    #[test]
    fn controller_failure_details_never_reach_action_outcomes() {
        let detail = "request /proxies/group-123e4567-e89b-12d3-a456-426614174000";
        let timeout = proxy_failure(ProxyCoreError::Timeout(detail.into()));
        let missing = proxy_failure(ProxyCoreError::NotFound(detail.into()));
        let unavailable = proxy_failure(ProxyCoreError::Unavailable(detail.into()));
        for failure in [&timeout, &missing, &unavailable] {
            assert!(!format!("{failure:?}").contains(detail));
        }
        assert!(matches!(
            timeout,
            FailureKind::Timeout { ref operation } if operation == "Mihomo controller request"
        ));
        assert!(matches!(
            missing,
            FailureKind::NotFound { ref entity } if entity == "Proxy item"
        ));
        assert!(matches!(
            unavailable,
            FailureKind::Unavailable { ref reason, retryable: true }
                if reason == "Mihomo controller is unavailable"
        ));
    }

    #[tokio::test]
    async fn fake_controller_timeout_is_redacted_from_action_outcome() {
        let (module, core, _) = module();
        let detail = "request /proxies/node-123e4567-e89b-12d3-a456-426614174000";
        core.set_error(Some(ProxyCoreError::Timeout(detail.into())))
            .await;
        let result = SearchItemDto {
            id: "proxy:mode:global".into(),
            module_id: MODULE_ID.into(),
            title: "Set Global mode".into(),
            kind: "proxy_mode".into(),
            primary_action_id: "set_global".into(),
            primary_action_label: "Set Global".into(),
            primary_action_risk: ActionRisk::Confirm,
            primary_action_confirmation: true,
            ..Default::default()
        }
        .into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result,
                    action: ActionDescriptor {
                        id: ActionId::new("set_global"),
                        label: "Set Global".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(!format!("{outcome:?}").contains(detail));
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::Timeout { ref operation }
            } if operation == "Mihomo controller request"
        ));
    }

    struct TestProfiles {
        summary: ProfileSummary,
    }

    #[async_trait]
    impl ProfileStorePort for TestProfiles {
        async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
            Ok(vec![self.summary.clone()])
        }
        async fn import_subscription(
            &self,
            _url: &str,
            _name: Option<&str>,
        ) -> Result<ProfileImportResult, ProfileStoreError> {
            Ok(ProfileImportResult {
                summary: self.summary.clone(),
                source_written: true,
                metadata_updated: true,
                runtime_applied: false,
            })
        }
        async fn import_local_file(
            &self,
            _path: &Path,
            _name: Option<&str>,
        ) -> Result<ProfileImportResult, ProfileStoreError> {
            Ok(ProfileImportResult {
                summary: self.summary.clone(),
                source_written: true,
                metadata_updated: true,
                runtime_applied: false,
            })
        }
        async fn use_profile(&self, _id: &str) -> Result<ProfileImportResult, ProfileStoreError> {
            Ok(ProfileImportResult {
                summary: self.summary.clone(),
                source_written: true,
                metadata_updated: true,
                runtime_applied: true,
            })
        }
        async fn refresh_profile(
            &self,
            _id: &str,
        ) -> Result<ProfileImportResult, ProfileStoreError> {
            Ok(ProfileImportResult {
                summary: self.summary.clone(),
                source_written: true,
                metadata_updated: true,
                runtime_applied: false,
            })
        }
        async fn delete_profile(&self, _id: &str) -> Result<(), ProfileStoreError> {
            Ok(())
        }
        async fn sync_convention_profile(&self) -> Result<ProfileImportResult, ProfileStoreError> {
            Ok(ProfileImportResult {
                summary: self.summary.clone(),
                source_written: true,
                metadata_updated: true,
                runtime_applied: false,
            })
        }
    }

    #[tokio::test]
    async fn standalone_status_uses_last_applied_luma_profile_name() {
        let (base, core, _) = module();
        core.status.lock().await.profile = None;
        let module = base.with_profile_store(Arc::new(TestProfiles {
            summary: ProfileSummary {
                id: "p-c0ffee0000000000000001".into(),
                name: "Personal VPS".into(),
                node_count: 1,
                group_count: 1,
                rule_count: 1,
                metadata_available: true,
                updated_at: Some(1),
                source: ProfileSource::LumaLocal,
                owned_by_luma: true,
                current: true,
            },
        }));
        let status = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:profile")
            .unwrap();
        assert!(status
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("Personal VPS")));
    }

    #[tokio::test]
    async fn profile_search_and_import_never_echo_subscription_url_or_credentials() {
        let (base, _, _) = module();
        let profiles = Arc::new(TestProfiles {
            summary: ProfileSummary {
                id: "p-0123456789abcdef0123".into(),
                name: "subscription-name".into(),
                node_count: 3,
                group_count: 1,
                rule_count: 2,
                metadata_available: true,
                updated_at: Some(1),
                source: ProfileSource::Subscription,
                owned_by_luma: true,
                current: false,
            },
        });
        let module = base.with_profile_store(profiles);
        let items = collect_search_items(&module, Query::parse("proxy profile", 20)).await;
        let serialized = format!("{items:?}");
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("uuid"));
        assert!(items.iter().any(|item| item.kind == "profile"));
        let import_items = collect_search_items(
            &module,
            Query::parse("proxy import https://example.invalid/token=secret", 20),
        )
        .await;
        let serialized = format!("{import_items:?}");
        assert!(!serialized.contains("https://example.invalid"));
        assert!(!serialized.contains("token=secret"));
        let refresh_items =
            collect_search_items(&module, Query::parse("proxy profile refresh", 20)).await;
        assert!(refresh_items
            .iter()
            .any(|item| item.primary_action.id.as_str() == "refresh_profile"));
    }

    #[tokio::test]
    async fn import_preserves_case_in_subscription_url_and_local_path() {
        let (base, _, _) = module();
        let profiles = Arc::new(TestProfiles {
            summary: ProfileSummary {
                id: "p-0123456789abcdef0123".into(),
                name: "subscription-name".into(),
                node_count: 0,
                group_count: 0,
                rule_count: 0,
                metadata_available: true,
                updated_at: None,
                source: ProfileSource::Subscription,
                owned_by_luma: true,
                current: false,
            },
        });
        let module = base.with_profile_store(profiles);
        let url = "https://Example.invalid/Profile/Case?Tag=AbC";
        let items =
            collect_search_items(&module, Query::parse(format!("proxy import {url}"), 20)).await;
        let intent = module
            .import_keys
            .read()
            .await
            .get(items[0].id.as_str())
            .cloned();
        assert!(matches!(intent, Some(ImportIntent::Subscription(value)) if value == url));

        let path = "/tmp/ProfileCase.YAML";
        let items =
            collect_search_items(&module, Query::parse(format!("proxy import {path}"), 20)).await;
        let intent = module
            .import_keys
            .read()
            .await
            .get(items[0].id.as_str())
            .cloned();
        assert!(matches!(intent, Some(ImportIntent::Local(value)) if value == *path));
    }

    #[test]
    fn external_clash_uid_is_never_exposed_in_result_id_payload_or_ui() {
        let uid = "external-profile-uid";
        let uuid = "123e4567-e89b-12d3-a456-426614174000";
        let item = profile_item(ProfileSummary {
            id: uid.into(),
            name: format!("{uid} {uuid}"),
            node_count: 0,
            group_count: 0,
            rule_count: 0,
            metadata_available: false,
            updated_at: None,
            source: ProfileSource::ClashVerge,
            owned_by_luma: false,
            current: false,
        });
        let serialized = serde_json::to_string(&item).unwrap();
        assert!(!serialized.contains(uid));
        assert!(!serialized.contains(uuid));
        assert!(item.id.starts_with("proxy:profile:readonly:"));
        assert_eq!(item.primary_action_id, "noop");
        assert!(item.secondary_actions.is_empty());
        assert!(item.action_payload.is_none());
    }

    #[test]
    fn only_subscription_profiles_offer_refresh() {
        let profile = |source| ProfileSummary {
            id: "p-0123456789abcdef0123".into(),
            name: "Safe Profile".into(),
            node_count: 0,
            group_count: 0,
            rule_count: 0,
            metadata_available: true,
            updated_at: None,
            source,
            owned_by_luma: true,
            current: false,
        };
        let local = profile_item(profile(ProfileSource::LumaLocal));
        let subscription = profile_item(profile(ProfileSource::Subscription));
        assert!(!local
            .secondary_actions
            .iter()
            .any(|action| action.id == "refresh_profile"));
        assert!(subscription
            .secondary_actions
            .iter()
            .any(|action| action.id == "refresh_profile"));
    }

    #[tokio::test]
    async fn sync_command_offers_confirmed_action_and_calls_store() {
        struct CountingProfiles {
            summary: ProfileSummary,
            syncs: tokio::sync::Mutex<u32>,
            uses: tokio::sync::Mutex<u32>,
        }
        #[async_trait]
        impl ProfileStorePort for CountingProfiles {
            async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
                Ok(vec![self.summary.clone()])
            }
            async fn import_subscription(
                &self,
                _url: &str,
                _name: Option<&str>,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn import_local_file(
                &self,
                _path: &Path,
                _name: Option<&str>,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn use_profile(
                &self,
                _id: &str,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                *self.uses.lock().await += 1;
                Ok(ProfileImportResult {
                    summary: self.summary.clone(),
                    source_written: true,
                    metadata_updated: true,
                    runtime_applied: true,
                })
            }
            async fn refresh_profile(
                &self,
                _id: &str,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn delete_profile(&self, _id: &str) -> Result<(), ProfileStoreError> {
                unreachable!()
            }
            async fn sync_convention_profile(
                &self,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                *self.syncs.lock().await += 1;
                Ok(ProfileImportResult {
                    summary: self.summary.clone(),
                    source_written: true,
                    metadata_updated: true,
                    runtime_applied: false,
                })
            }
        }
        let (base, _, _) = module();
        let profiles = Arc::new(CountingProfiles {
            summary: ProfileSummary {
                id: "p-c0ffee0000000000000001".into(),
                name: "Personal VPS".into(),
                node_count: 2,
                group_count: 1,
                rule_count: 1,
                metadata_available: true,
                updated_at: Some(1),
                source: ProfileSource::LumaLocal,
                owned_by_luma: true,
                current: false,
            },
            syncs: tokio::sync::Mutex::new(0),
            uses: tokio::sync::Mutex::new(0),
        });
        let module = base.with_profile_store(profiles.clone());
        let items = collect_search_items(&module, Query::parse("proxy sync", 20)).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "profile_sync");
        assert_eq!(
            items[0].primary_action.id.as_str(),
            "sync_convention_profile"
        );
        let actions = module.actions(&items[0]).await;
        let outcome = module
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: actions[0].clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::Success {
                message: Some(message),
            } => {
                assert!(message.contains("2"));
                assert!(message.contains("尚未应用到运行中的 Mihomo"));
                assert!(!message.contains("password"));
                assert!(!message.contains("uuid"));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
        assert_eq!(*profiles.syncs.lock().await, 1);
        assert_eq!(*profiles.uses.lock().await, 0);

        let items = collect_search_items(&module, Query::parse("proxy apply", 20)).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "profile_apply");
        assert_eq!(
            items[0].primary_action.id.as_str(),
            "apply_convention_profile"
        );
        let action = module.actions(&items[0]).await.remove(0);
        let outcome = module
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action,
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(*profiles.syncs.lock().await, 2);
        assert_eq!(*profiles.uses.lock().await, 1);
    }

    #[tokio::test]
    async fn sync_invalid_errors_expose_field_names_only() {
        struct FailingSync;
        #[async_trait]
        impl ProfileStorePort for FailingSync {
            async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
                Ok(vec![])
            }
            async fn import_subscription(
                &self,
                _url: &str,
                _name: Option<&str>,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn import_local_file(
                &self,
                _path: &Path,
                _name: Option<&str>,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn use_profile(
                &self,
                _id: &str,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn refresh_profile(
                &self,
                _id: &str,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                unreachable!()
            }
            async fn delete_profile(&self, _id: &str) -> Result<(), ProfileStoreError> {
                unreachable!()
            }
            async fn sync_convention_profile(
                &self,
            ) -> Result<ProfileImportResult, ProfileStoreError> {
                Err(ProfileStoreError::InvalidInput {
                    field: "uuid".into(),
                    message: "uuid must be a valid UUID".into(),
                })
            }
        }
        let (base, _, _) = module();
        let module = base.with_profile_store(Arc::new(FailingSync));
        let items = collect_search_items(&module, Query::parse("proxy sync", 20)).await;
        let actions = module.actions(&items[0]).await;
        let outcome = module
            .perform(
                ActionRequest {
                    result: items[0].clone(),
                    action: actions[0].clone(),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::Failed {
                kind: FailureKind::InvalidInput { field, message },
            } => {
                assert_eq!(field, "uuid");
                assert!(!message.contains("00000000"));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[tokio::test]
    async fn enabling_system_proxy_uses_mixed_port_when_dedicated_ports_are_absent() {
        let (module, core, system) = module();
        core.status.lock().await.ports = ProxyPorts {
            http: None,
            mixed: Some(7897),
            socks: None,
        };
        let status = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.kind == "proxy_system_setting")
            .unwrap();
        let action = module
            .actions(&status)
            .await
            .into_iter()
            .find(|action| action.id.as_str() == "enable_system_proxy")
            .unwrap();
        let outcome = module
            .perform(
                ActionRequest {
                    result: status,
                    action,
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(
            *system.enable_calls.lock().await,
            vec![(Some(7897), Some(7897), Some(7897))]
        );
    }

    #[tokio::test]
    async fn enabling_system_proxy_stops_before_mutation_when_listener_is_down() {
        let (base, _, system) = module();
        let probe = FakeNetworkProbe::new(vec![]);
        probe.listeners.lock().await.insert(
            7899,
            NetworkProbeStep {
                name: "Local listener 127.0.0.1:7899".into(),
                state: NetworkProbeState::Fail,
                detail: "not accepting connections".into(),
                remediation: "start Mihomo".into(),
            },
        );
        let module = base.with_network_probe(probe);
        let item = collect_search_items(&module, Query::parse("proxy ", 20))
            .await
            .into_iter()
            .find(|item| item.id.as_str() == "proxy:setting:system")
            .unwrap();
        let action = module.actions(&item).await.remove(0);
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action,
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::Unavailable { .. }
            }
        ));
        assert!(system.enable_calls.lock().await.is_empty());
    }
}
