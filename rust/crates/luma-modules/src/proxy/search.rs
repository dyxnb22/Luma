use super::profiles::{action_dto, profile_error_item, profile_item, profile_unavailable};
use super::redact::{opaque_component, redact_label};
use super::{ImportIntent, ProxyModule, MODULE_ID};
use crate::cancel::await_unless_cancelled;
use luma_application::{
    ProfileSource, ProxyCoreError, ProxyGroup, ProxyMode, ProxyNode, ProxyStatus, SearchSink,
};
use luma_domain::{ActionRisk, Query};
use luma_protocol::{Event, SearchItemDto};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

impl ProxyModule {
    pub(super) fn status_item(
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

    pub(super) fn unavailable_item(error: &ProxyCoreError) -> SearchItemDto {
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

    pub(super) fn group_item(group: &ProxyGroup, score: f64) -> SearchItemDto {
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

    pub(super) fn node_item(group: &str, node: &ProxyNode, score: f64) -> SearchItemDto {
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

    pub(super) async fn search_ready(
        &self,
        query: &Query,
        sink: &SearchSink,
        cancel: &CancellationToken,
    ) {
        let normalized_rest = query.rest_normalized();
        if normalized_rest == "profile"
            || normalized_rest.starts_with("profile ")
            || normalized_rest == "import"
            || normalized_rest.starts_with("import ")
        {
            self.search_profiles(&normalized_rest, query.rest_raw(), query.limit, sink)
                .await;
            return;
        }
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
        let mut items = vec![Self::status_item(&status, system.as_ref())];
        if normalized_rest == "global" || normalized_rest == "rule" {
            let mode = if normalized_rest == "global" {
                "global"
            } else {
                "rule"
            };
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
            let requested_group = normalized_rest
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

    pub(super) async fn search_profiles(
        &self,
        normalized_rest: &str,
        raw_rest: &str,
        limit: usize,
        sink: &SearchSink,
    ) {
        let Some(store) = &self.profiles else {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![profile_unavailable()],
                    removed_ids: vec![],
                })
                .await;
            return;
        };
        if normalized_rest == "import" || normalized_rest.starts_with("import ") {
            // Keep only the current import intent — previous browse entries must not accumulate.
            self.import_keys.write().await.clear();
            let source = raw_rest
                .split_once(|character: char| character.is_whitespace())
                .map(|(_, source)| source.trim())
                .unwrap_or("");
            let item = if source.is_empty() {
                SearchItemDto {
                    id: "proxy:profile:import-help".into(),
                    module_id: MODULE_ID.into(),
                    title: "Import a Profile".into(),
                    subtitle: Some("Use /proxy import <HTTPS URL or local YAML path>".into()),
                    kind: "profile_import_help".into(),
                    score: 95.0,
                    ..Default::default()
                }
            } else {
                let id = format!("proxy:profile:import:{}", opaque_component(source));
                let intent = if source.starts_with("https://") || source.starts_with("http://") {
                    ImportIntent::Subscription(source.to_string())
                } else {
                    ImportIntent::Local(PathBuf::from(source))
                };
                self.import_keys.write().await.insert(id.clone(), intent);
                SearchItemDto {
                    id,
                    module_id: MODULE_ID.into(),
                    title: if source.starts_with("http") {
                        "Import HTTPS subscription".into()
                    } else {
                        "Import local YAML".into()
                    },
                    subtitle: Some(
                        "Source hidden until import; YAML will be validated before saving".into(),
                    ),
                    kind: "profile_import".into(),
                    score: 95.0,
                    primary_action_id: "import_profile".into(),
                    primary_action_label: "Import".into(),
                    primary_action_risk: ActionRisk::Confirm,
                    primary_action_confirmation: true,
                    ..Default::default()
                }
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![item],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        let filter = normalized_rest
            .strip_prefix("profile")
            .unwrap_or("")
            .trim()
            .to_lowercase();
        match store.list_profiles().await {
            Ok(profiles) => {
                let refresh_mode = filter == "refresh";
                let items = profiles
                    .into_iter()
                    .filter(|p| {
                        (refresh_mode && p.owned_by_luma && p.source == ProfileSource::Subscription)
                            || (!refresh_mode
                                && (filter.is_empty() || p.name.to_lowercase().contains(&filter)))
                    })
                    .take(limit)
                    .map(|profile| {
                        let mut item = profile_item(profile);
                        if refresh_mode && item.kind == "profile" {
                            item.primary_action_id = "refresh_profile".into();
                            item.primary_action_label = "Refresh".into();
                            item.primary_action_risk = ActionRisk::Safe;
                            item.primary_action_confirmation = false;
                        }
                        item
                    })
                    .collect();
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: items,
                        removed_ids: vec!["proxy:profile:unavailable".into()],
                    })
                    .await;
            }
            Err(error) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![profile_error_item(&error)],
                        removed_ids: vec![],
                    })
                    .await;
            }
        }
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
