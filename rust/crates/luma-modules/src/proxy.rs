//! Mihomo controller-first proxy module for local macOS use.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode, ProxyNode, ProxyStatus, SearchMode,
    SearchSink, SystemProxyError, SystemProxyPort, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.proxy";

pub struct ProxyModule {
    manifest: ModuleManifest,
    core: Arc<dyn ProxyCorePort>,
    system_proxy: Arc<dyn SystemProxyPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    last_status: RwLock<Option<ProxyStatus>>,
    selection_keys: RwLock<HashMap<String, (String, String)>>,
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
                    suggested_query: Some("proxy ".into()),
                    empty_hint: Some("proxy · inspect Mihomo and system proxy".into()),
                    supports_browse: true,
                },
            },
            core,
            system_proxy,
            pasteboard,
            last_status: RwLock::new(None),
            selection_keys: RwLock::new(HashMap::new()),
        }
    }

    fn status_item(
        status: &ProxyStatus,
        system: Option<&luma_application::SystemProxyStatus>,
    ) -> SearchItemDto {
        let mode = mode_label(status.mode);
        let system_label = system
            .map(|s| {
                if s.http.enabled || s.socks.enabled {
                    "ON"
                } else {
                    "OFF"
                }
            })
            .unwrap_or("unavailable");
        let mut parts = vec![format!(
            "Profile: {}",
            status
                .profile
                .as_deref()
                .map(redact_label)
                .unwrap_or_else(|| "unknown".into())
        )];
        if let Some(port) = status.ports.http {
            parts.push(format!("HTTP: {port}"));
        }
        if let Some(port) = status.ports.mixed {
            parts.push(format!("Mixed: {port}"));
        }
        if let Some(port) = status.ports.socks {
            parts.push(format!("SOCKS: {port}"));
        }
        parts.push(format!("System proxy: {system_label}"));
        parts.push("Mihomo: connected".into());
        let address = status
            .ports
            .mixed
            .or(status.ports.http)
            .map(|port| format!("127.0.0.1:{port}"));
        SearchItemDto {
            id: "proxy:status".into(),
            module_id: MODULE_ID.into(),
            title: format!("Proxy running · {mode}"),
            subtitle: Some(parts.join(" · ")),
            kind: "status".into(),
            score: 100.0,
            primary_action_id: "refresh".into(),
            primary_action_label: "Refresh".into(),
            secondary_actions: status_actions(system, address.is_some()),
            action_payload: address.map(|address| serde_json::json!({ "address": address })),
            ..Default::default()
        }
    }

    fn unavailable_item(error: &ProxyCoreError) -> SearchItemDto {
        let (kind, title, subtitle) = match error {
            ProxyCoreError::PermissionRequired(guidance) => (
                "permission_required",
                "Mihomo permission required",
                guidance.clone(),
            ),
            ProxyCoreError::Timeout(_) => (
                "unavailable",
                "Mihomo controller timed out",
                "Check that Clash Verge/Mihomo is running, then refresh".into(),
            ),
            _ => (
                "unavailable",
                "Mihomo unavailable",
                "Start Mihomo or Clash Verge, then refresh".into(),
            ),
        };
        SearchItemDto {
            id: "proxy:unavailable".into(),
            module_id: MODULE_ID.into(),
            title: title.into(),
            subtitle: Some(subtitle),
            kind: kind.into(),
            primary_action_id: "refresh".into(),
            primary_action_label: "Refresh".into(),
            ..Default::default()
        }
    }

    fn group_item(group: &ProxyGroup, score: f64) -> SearchItemDto {
        let selected = group
            .selected
            .as_deref()
            .map(redact_label)
            .unwrap_or_else(|| "none".into());
        SearchItemDto {
            id: format!("proxy:group:{}", opaque_component(&group.name)),
            module_id: MODULE_ID.into(),
            title: redact_label(&group.name),
            subtitle: Some(format!("Selected: {selected}")),
            kind: "proxy_group".into(),
            score,
            primary_action_id: "noop".into(),
            primary_action_label: "OK".into(),
            ..Default::default()
        }
    }

    fn node_item(group: &str, node: &ProxyNode, score: f64) -> SearchItemDto {
        let delay = node
            .delay_ms
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| "delay unavailable".into());
        SearchItemDto {
            id: format!(
                "proxy:node:{}",
                opaque_component(&format!("{group}\0{}", node.name))
            ),
            module_id: MODULE_ID.into(),
            title: redact_label(&node.name),
            subtitle: Some(format!(
                "{} · {delay} · {}",
                redact_label(&node.kind),
                if node.selected {
                    "selected"
                } else {
                    "not selected"
                }
            )),
            kind: "proxy_node".into(),
            score,
            primary_action_id: "select_proxy".into(),
            primary_action_label: if node.selected {
                "Selected".into()
            } else {
                "Select".into()
            },
            primary_action_risk: ActionRisk::Safe,
            primary_action_confirmation: false,
            ..Default::default()
        }
    }

    async fn search_ready(&self, query: &Query, sink: &SearchSink, cancel: &CancellationToken) {
        let status = match await_unless_cancelled(cancel, self.core.get_status()).await {
            None => return,
            Some(Ok(status)) => status,
            Some(Err(error)) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![Self::unavailable_item(&error)],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        self.selection_keys.write().await.clear();
        *self.last_status.write().await = Some(status.clone());
        let system = self.system_proxy.get_status().await.ok();
        let groups = match await_unless_cancelled(cancel, self.core.list_proxy_groups()).await {
            None => return,
            Some(Ok(groups)) => groups,
            Some(Err(error)) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![
                            Self::status_item(&status, system.as_ref()),
                            Self::unavailable_item(&error),
                        ],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        let rest = query.rest_normalized();
        let mut items = vec![Self::status_item(&status, system.as_ref())];
        if rest == "global" || rest == "rule" {
            let mode = if rest == "global" { "global" } else { "rule" };
            items.push(SearchItemDto {
                id: format!("proxy:mode:{mode}"),
                module_id: MODULE_ID.into(),
                title: format!(
                    "Set {} mode",
                    if mode == "global" { "Global" } else { "Rule" }
                ),
                subtitle: Some("Changes Mihomo traffic routing mode".into()),
                kind: "proxy_mode".into(),
                score: 95.0,
                primary_action_id: format!("set_{mode}"),
                primary_action_label: format!(
                    "Set {}",
                    if mode == "global" { "Global" } else { "Rule" }
                ),
                primary_action_risk: ActionRisk::Confirm,
                primary_action_confirmation: true,
                ..Default::default()
            });
        } else {
            let requested_group = rest
                .strip_prefix("group ")
                .map(str::trim)
                .filter(|v| !v.is_empty());
            for group in &groups {
                if requested_group
                    .map(|needle| !group.name.to_lowercase().contains(needle))
                    .unwrap_or(false)
                {
                    continue;
                }
                items.push(Self::group_item(
                    group,
                    if requested_group.is_some() {
                        93.0
                    } else {
                        80.0
                    },
                ));
                if requested_group.is_some() {
                    for node in &group.nodes {
                        let item = Self::node_item(
                            &group.name,
                            node,
                            if node.selected { 92.0 } else { 70.0 },
                        );
                        self.selection_keys
                            .write()
                            .await
                            .insert(item.id.clone(), (group.name.clone(), node.name.clone()));
                        items.push(item);
                    }
                } else if let Some(node) = group.nodes.iter().find(|node| node.selected) {
                    let item = Self::node_item(&group.name, node, 88.0);
                    self.selection_keys
                        .write()
                        .await
                        .insert(item.id.clone(), (group.name.clone(), node.name.clone()));
                    items.push(item);
                }
            }
        }
        items.truncate(query.limit);
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: items,
                removed_ids: vec!["proxy:unavailable".into()],
            })
            .await;
    }
}

fn mode_label(mode: ProxyMode) -> &'static str {
    match mode {
        ProxyMode::Global => "Global",
        ProxyMode::Rule => "Rule",
    }
}

fn status_actions(
    system: Option<&luma_application::SystemProxyStatus>,
    can_copy: bool,
) -> Vec<luma_protocol::ActionDescriptorDto> {
    let system_on = system
        .map(|s| s.http.enabled || s.socks.enabled)
        .unwrap_or(false);
    let mut actions = vec![
        action_dto("set_global", "Set Global", ActionRisk::Confirm, true),
        action_dto("set_rule", "Set Rule", ActionRisk::Confirm, true),
        action_dto("refresh", "Refresh", ActionRisk::Safe, false),
        action_dto(
            if system_on {
                "disable_system_proxy"
            } else {
                "enable_system_proxy"
            },
            if system_on {
                "Disable System Proxy"
            } else {
                "Enable System Proxy"
            },
            ActionRisk::Confirm,
            true,
        ),
    ];
    if can_copy {
        actions.push(action_dto(
            "copy_proxy_address",
            "Copy Proxy Address",
            ActionRisk::Safe,
            false,
        ));
    }
    actions
}

fn action_dto(
    id: &str,
    label: &str,
    risk: ActionRisk,
    confirmation: bool,
) -> luma_protocol::ActionDescriptorDto {
    luma_protocol::ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
    }
}

fn opaque_component(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Avoid exposing obvious credential-shaped values that a provider may have put into a label.
fn redact_label(value: &str) -> String {
    if value.contains("://") || looks_like_uuid(value) {
        return "[redacted]".into();
    }
    value.to_string()
}

fn looks_like_uuid(value: &str) -> bool {
    const UUID_LEN: usize = 36;
    if value.len() < UUID_LEN {
        return false;
    }
    (0..=value.len() - UUID_LEN).any(|start| {
        let Some(candidate) = value.get(start..start + UUID_LEN) else {
            return false;
        };
        let groups: Vec<_> = candidate.split('-').collect();
        groups.len() == 5
            && [8, 4, 4, 4, 12]
                .iter()
                .zip(groups)
                .all(|(want, got)| got.len() == *want && got.chars().all(|c| c.is_ascii_hexdigit()))
    })
}

fn proxy_failure(error: ProxyCoreError) -> FailureKind {
    match error {
        ProxyCoreError::PermissionRequired(guidance) => FailureKind::PermissionRequired {
            capability: "mihomo_controller".into(),
            guidance,
        },
        ProxyCoreError::Timeout(operation) => FailureKind::Timeout { operation },
        ProxyCoreError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProxyCoreError::NotFound(entity) => FailureKind::NotFound { entity },
        ProxyCoreError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        ProxyCoreError::NotConfigured(remediation) => FailureKind::NotConfigured { remediation },
        ProxyCoreError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
    }
}

fn system_failure(error: SystemProxyError) -> FailureKind {
    match error {
        SystemProxyError::PermissionRequired(guidance) => FailureKind::PermissionRequired {
            capability: "system_proxy".into(),
            guidance,
        },
        SystemProxyError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        SystemProxyError::Conflict => FailureKind::Conflict {
            reason: "System proxy changed outside Luma; it was not overwritten".into(),
        },
        SystemProxyError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
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
            "unavailable" | "permission_required" => vec![ActionDescriptor {
                id: ActionId::new("refresh"),
                label: "Refresh".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "proxy_node" => vec![ActionDescriptor {
                id: ActionId::new("select_proxy"),
                label: if result.primary_action.id.as_str() == "select_proxy"
                    && result.primary_action.label == "Selected"
                {
                    "Selected"
                } else {
                    "Select"
                }
                .into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "proxy_group" => vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "proxy_mode" => {
                let global = result.primary_action.id.as_str() == "set_global";
                vec![ActionDescriptor {
                    id: ActionId::new(if global { "set_global" } else { "set_rule" }),
                    label: if global { "Set Global" } else { "Set Rule" }.into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                }]
            }
            _ => {
                let system_on = self
                    .system_proxy
                    .get_status()
                    .await
                    .map(|s| s.http.enabled || s.socks.enabled)
                    .unwrap_or(false);
                let mut actions = vec![
                    ActionDescriptor {
                        id: ActionId::new("set_global"),
                        label: "Set Global".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: true,
                    },
                    ActionDescriptor {
                        id: ActionId::new("set_rule"),
                        label: "Set Rule".into(),
                        risk: ActionRisk::Confirm,
                        confirmation: true,
                    },
                    ActionDescriptor {
                        id: ActionId::new("refresh"),
                        label: "Refresh".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    ActionDescriptor {
                        id: ActionId::new(if system_on {
                            "disable_system_proxy"
                        } else {
                            "enable_system_proxy"
                        }),
                        label: if system_on {
                            "Disable System Proxy"
                        } else {
                            "Enable System Proxy"
                        }
                        .into(),
                        risk: ActionRisk::Confirm,
                        confirmation: true,
                    },
                ];
                if result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("address"))
                    .and_then(Value::as_str)
                    .is_some()
                {
                    actions.push(ActionDescriptor {
                        id: ActionId::new("copy_proxy_address"),
                        label: "Copy Proxy Address".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    });
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
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let id = action.action.id.as_str();
        match id {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "set_global" => match self.core.set_mode(ProxyMode::Global).await {
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
            "refresh" => match self.core.refresh_provider().await {
                Ok(()) => ActionOutcome::Success {
                    message: Some("proxy providers refreshed".into()),
                },
                Err(error) => ActionOutcome::Failed {
                    kind: proxy_failure(error),
                },
            },
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
            "enable_system_proxy" => {
                let status = match self.core.get_status().await {
                    Ok(status) => status,
                    Err(error) => {
                        return ActionOutcome::Failed {
                            kind: proxy_failure(error),
                        }
                    }
                };
                match self
                    .system_proxy
                    .enable(status.ports.http, status.ports.socks)
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
                match self.pasteboard.write_text(address).await {
                    Ok(()) => {
                        return ActionOutcome::Success {
                            message: Some("proxy address copied".into()),
                        }
                    }
                    Err(error) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason: error.to_string(),
                                retryable: true,
                            },
                        }
                    }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakePasteboard, FakeProxyCore, FakeSystemProxy, ProxyPorts, SystemProxySetting,
        SystemProxyStatus,
    };
    use luma_test_support::collect_search_items;

    fn module() -> (ProxyModule, Arc<FakeProxyCore>) {
        let core = FakeProxyCore::new(
            ProxyStatus {
                running: true,
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
            socks: SystemProxySetting::default(),
        });
        let module = ProxyModule::with_deps(core.clone(), system, Arc::new(FakePasteboard::new()));
        (module, core)
    }

    #[tokio::test]
    async fn search_redacts_uuid_and_shows_status_and_selected_node() {
        let (module, _) = module();
        let items = collect_search_items(&module, Query::parse("proxy ", 20)).await;
        assert!(items
            .iter()
            .any(|item| item.title == "Proxy running · Rule"));
        assert!(items.iter().any(|item| item.title == "V2Box-VPS"
            && item.subtitle.as_deref().unwrap().contains("selected")));
        assert!(!redact_label("node-123e4567-e89b-12d3-a456-426614174000").contains("123e4567"));
    }

    #[tokio::test]
    async fn selecting_node_is_safe_and_calls_core() {
        let (module, core) = module();
        let items = collect_search_items(&module, Query::parse("proxy group AI-VPS", 20)).await;
        let node = items
            .iter()
            .find(|item| item.kind == "proxy_node")
            .unwrap()
            .clone();
        let actions = module.actions(&node).await;
        assert_eq!(actions[0].id.as_str(), "select_proxy");
        let outcome = module
            .perform(
                ActionRequest {
                    result: node,
                    action: actions[0].clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(core.selected.lock().await.len(), 1);
    }
}
