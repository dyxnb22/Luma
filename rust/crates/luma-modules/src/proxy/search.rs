use super::profiles::{action_dto, profile_error_item, profile_item, profile_unavailable};
use super::redact::{opaque_component, redact_label};
use super::{ImportIntent, ProxyModule, MODULE_ID};
use crate::cancel::await_unless_cancelled;
use luma_application::{
    NetworkProbeState, ProfileSource, ProxyCoreError, ProxyGroup, ProxyMode, ProxyNode,
    ProxyStatus, SearchSink, SystemProxySetting, SystemProxyStatus,
};
use luma_domain::{ActionRisk, Query};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SystemProxyState {
    Off,
    On,
    Mismatch,
    Unavailable,
}

impl SystemProxyState {
    fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::On => "ON",
            Self::Mismatch => "MISMATCH",
            Self::Unavailable => "unavailable",
        }
    }
}

pub(super) fn system_proxy_state(
    status: &ProxyStatus,
    system: Option<&SystemProxyStatus>,
) -> SystemProxyState {
    let Some(system) = system else {
        return SystemProxyState::Unavailable;
    };
    let expected_http = status.ports.http.or(status.ports.mixed);
    let expected_https = expected_http;
    let expected_socks = status.ports.socks.or(status.ports.mixed);
    if expected_http.is_none() && expected_socks.is_none() {
        return SystemProxyState::Unavailable;
    }
    if !system.http.enabled && !system.https.enabled && !system.socks.enabled {
        return SystemProxyState::Off;
    }

    let matches = |setting: &SystemProxySetting, expected: Option<u16>| match expected {
        Some(port) => {
            setting.enabled
                && setting.port == Some(port)
                && setting
                    .server
                    .as_deref()
                    .is_some_and(|server| matches!(server, "127.0.0.1" | "localhost" | "::1"))
        }
        None => !setting.enabled,
    };
    if matches(&system.http, expected_http)
        && matches(&system.https, expected_https)
        && matches(&system.socks, expected_socks)
    {
        SystemProxyState::On
    } else {
        SystemProxyState::Mismatch
    }
}

impl ProxyModule {
    fn check_row(step: luma_application::NetworkProbeStep) -> SearchItemDto {
        let state = match step.state {
            NetworkProbeState::Pass => "pass",
            NetworkProbeState::Fail => "fail",
            NetworkProbeState::Skipped => "skipped",
        };
        SearchItemDto {
            id: format!("proxy:check:{}", opaque_component(&step.name)),
            module_id: MODULE_ID.into(),
            title: format!("{state} · {}", step.name),
            subtitle: Some(format!("{} · {}", step.detail, step.remediation)),
            kind: "status".into(),
            score: if matches!(step.state, NetworkProbeState::Fail) {
                95.0
            } else {
                70.0
            },
            primary_action_id: "rerun_proxy_check".into(),
            primary_action_label: "Run Again".into(),
            ui_intent: Some(UiIntent::OpenSurface),
            ..Default::default()
        }
    }

    async fn status_rows(
        &self,
        status: &ProxyStatus,
        system: Option<&luma_application::SystemProxyStatus>,
    ) -> Vec<SearchItemDto> {
        let mut rows = vec![Self::status_item(status, system)];
        if let Some(system) = system {
            let setting = |name: &str, value: &SystemProxySetting| {
                let endpoint = match (value.server.as_deref(), value.port) {
                    (Some(server), Some(port)) => format!("{server}:{port}"),
                    (Some(server), None) => server.to_string(),
                    (None, Some(port)) => format!("port {port}"),
                    (None, None) => "not set".into(),
                };
                SearchItemDto {
                    id: format!("proxy:system:{name}"),
                    module_id: MODULE_ID.into(),
                    title: format!(
                        "System {name} proxy · {}",
                        if value.enabled { "on" } else { "off" }
                    ),
                    subtitle: Some(format!("{} · {}", system.service, endpoint)),
                    kind: "status".into(),
                    score: 85.0,
                    primary_action_id: "open_proxy_overview".into(),
                    primary_action_label: "Overview".into(),
                    ui_intent: Some(UiIntent::OpenSurface),
                    ..Default::default()
                }
            };
            rows.push(setting("HTTP", &system.http));
            rows.push(setting("HTTPS", &system.https));
            rows.push(setting("SOCKS", &system.socks));
        } else {
            rows.push(SearchItemDto {
                id: "proxy:system:unavailable".into(),
                module_id: MODULE_ID.into(),
                title: "System proxy status unavailable".into(),
                subtitle: Some("Check macOS network service permissions".into()),
                kind: "unavailable".into(),
                primary_action_id: "open_proxy_overview".into(),
                primary_action_label: "Overview".into(),
                ui_intent: Some(UiIntent::OpenSurface),
                ..Default::default()
            });
        }
        match self.core.get_external_controller_status().await {
            Ok(controller) => rows.push(SearchItemDto {
                id: "proxy:controller".into(),
                module_id: MODULE_ID.into(),
                title: format!(
                    "Mihomo controller · {}",
                    if controller.connected {
                        "available"
                    } else {
                        "unavailable"
                    }
                ),
                subtitle: Some("local controller only".into()),
                kind: "status".into(),
                score: 84.0,
                primary_action_id: "open_proxy_overview".into(),
                primary_action_label: "Overview".into(),
                ui_intent: Some(UiIntent::OpenSurface),
                ..Default::default()
            }),
            Err(error) => rows.push(Self::unavailable_item(&error)),
        }
        if let Some(profiles) = &self.profiles {
            match profiles.list_profiles().await {
                Ok(items) => {
                    let owned = items.iter().filter(|profile| profile.owned_by_luma).count();
                    let current = items
                        .iter()
                        .filter(|profile| profile.current && profile.owned_by_luma)
                        .count();
                    rows.push(SearchItemDto {
                        id: "proxy:profiles:owned".into(),
                        module_id: MODULE_ID.into(),
                        title: format!("Luma-owned profiles · {owned}"),
                        subtitle: Some(format!(
                            "active {current} · non-Luma Clash Verge profiles stay read-only"
                        )),
                        kind: "status".into(),
                        score: 83.0,
                        primary_action_id: "open_proxy_overview".into(),
                        primary_action_label: "Overview".into(),
                        ui_intent: Some(UiIntent::OpenSurface),
                        ..Default::default()
                    });
                }
                Err(_) => rows.push(SearchItemDto {
                    id: "proxy:profiles:unavailable".into(),
                    module_id: MODULE_ID.into(),
                    title: "Luma profile status unavailable".into(),
                    subtitle: Some("Existing profiles were left untouched".into()),
                    kind: "unavailable".into(),
                    primary_action_id: "open_proxy_overview".into(),
                    primary_action_label: "Overview".into(),
                    ui_intent: Some(UiIntent::OpenSurface),
                    ..Default::default()
                }),
            }
        }
        rows
    }

    async fn check_rows(
        &self,
        status: &ProxyStatus,
        system: Option<&luma_application::SystemProxyStatus>,
    ) -> Vec<SearchItemDto> {
        let mut rows = self
            .network_probe
            .base_checks()
            .await
            .into_iter()
            .map(Self::check_row)
            .collect::<Vec<_>>();
        let mut ports = vec![status.ports.http, status.ports.mixed, status.ports.socks];
        if let Some(system) = system {
            for (name, setting) in [
                ("HTTP", &system.http),
                ("HTTPS", &system.https),
                ("SOCKS", &system.socks),
            ] {
                if !setting.enabled {
                    continue;
                }
                let loopback = setting
                    .server
                    .as_deref()
                    .is_some_and(|server| matches!(server, "127.0.0.1" | "localhost" | "::1"));
                if loopback {
                    if let Some(port) = setting.port {
                        ports.push(Some(port));
                    } else {
                        rows.push(Self::check_row(luma_application::NetworkProbeStep {
                            name: format!("System {name} proxy"),
                            state: NetworkProbeState::Fail,
                            detail: "enabled without a port".into(),
                            remediation: "Set a valid local proxy port or turn this proxy off"
                                .into(),
                        }));
                    }
                } else {
                    rows.push(Self::check_row(luma_application::NetworkProbeStep {
                        name: format!("System {name} proxy"),
                        state: NetworkProbeState::Skipped,
                        detail: "points to a non-local address".into(),
                        remediation: "Use a local listener or verify that proxy outside Luma"
                            .into(),
                    }));
                }
            }
        }
        ports.sort_unstable();
        ports.dedup();
        for port in ports.into_iter().flatten() {
            rows.push(Self::check_row(
                self.network_probe.loopback_listener(port).await,
            ));
        }
        match self.core.get_external_controller_status().await {
            Ok(status) => rows.push(SearchItemDto {
                id: "proxy:check:controller".into(),
                module_id: MODULE_ID.into(),
                title: format!(
                    "{} · Mihomo controller",
                    if status.connected { "pass" } else { "fail" }
                ),
                subtitle: Some("controller reachability".into()),
                kind: "status".into(),
                score: if status.connected { 70.0 } else { 95.0 },
                primary_action_id: "rerun_proxy_check".into(),
                primary_action_label: "Run Again".into(),
                ui_intent: Some(UiIntent::OpenSurface),
                ..Default::default()
            }),
            Err(error) => rows.push(Self::unavailable_item(&error)),
        }
        rows
    }

    pub(super) fn status_item(
        status: &ProxyStatus,
        system: Option<&luma_application::SystemProxyStatus>,
    ) -> SearchItemDto {
        let mode = mode_label(status.mode);
        let system_state = system_proxy_state(status, system);
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
        parts.push(format!("System proxy: {}", system_state.label()));
        parts.push("Mihomo: connected".into());
        let address = status
            .ports
            .mixed
            .or(status.ports.http)
            .map(|port| format!("127.0.0.1:{port}"));
        let (primary_action_id, primary_action_label, primary_risk, primary_confirmation) =
            match system_state {
                SystemProxyState::Off => (
                    "enable_system_proxy",
                    "Enable System Proxy",
                    ActionRisk::Safe,
                    false,
                ),
                SystemProxyState::Mismatch => (
                    "enable_system_proxy",
                    "Switch System Proxy",
                    ActionRisk::Safe,
                    false,
                ),
                SystemProxyState::On | SystemProxyState::Unavailable => {
                    ("open_proxy_status", "Details", ActionRisk::Safe, false)
                }
            };
        SearchItemDto {
            id: "proxy:status".into(),
            module_id: MODULE_ID.into(),
            title: format!("Proxy running · {mode}"),
            subtitle: Some(parts.join(" · ")),
            kind: "status".into(),
            score: 100.0,
            primary_action_id: primary_action_id.into(),
            primary_action_label: primary_action_label.into(),
            primary_action_risk: primary_risk,
            primary_action_confirmation: primary_confirmation,
            secondary_actions: status_actions(system_state, address.is_some()),
            ui_intent: matches!(
                system_state,
                SystemProxyState::On | SystemProxyState::Unavailable
            )
            .then_some(UiIntent::OpenSurface),
            action_payload: address.map(|address| serde_json::json!({ "address": address })),
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
            primary_action_id: "retry_proxy".into(),
            primary_action_label: "Retry".into(),
            ui_intent: Some(UiIntent::OpenSurface),
            ..Default::default()
        }
    }

    pub(super) fn group_item(group: &ProxyGroup, mode: ProxyMode, score: f64) -> SearchItemDto {
        let selected = group
            .selected
            .as_deref()
            .map(redact_label)
            .unwrap_or_else(|| "none".into());
        let activity = if group.name.eq_ignore_ascii_case("GLOBAL") && mode == ProxyMode::Rule {
            " · inactive in Rule mode"
        } else {
            ""
        };
        SearchItemDto {
            id: format!("proxy:group:{}", opaque_component(&group.name)),
            module_id: MODULE_ID.into(),
            title: redact_label(&group.name),
            subtitle: Some(format!(
                "Selected: {selected} · {} choices{activity}",
                group.nodes.len()
            )),
            kind: "proxy_group".into(),
            score,
            primary_action_id: "open_proxy_group".into(),
            primary_action_label: "Open".into(),
            ui_intent: Some(UiIntent::OpenSurface),
            ..Default::default()
        }
    }

    pub(super) fn node_item(group: &str, node: &ProxyNode, score: f64) -> SearchItemDto {
        let delay = node
            .delay_ms
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| "delay unavailable".into());
        let (primary_action_id, primary_action_label, secondary_actions) = if node.selected {
            ("test_proxy", "Test", vec![])
        } else {
            (
                "select_proxy",
                "Select",
                vec![action_dto("test_proxy", "Test", ActionRisk::Safe, false)],
            )
        };
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
            primary_action_id: primary_action_id.into(),
            primary_action_label: primary_action_label.into(),
            primary_action_risk: ActionRisk::Safe,
            primary_action_confirmation: false,
            secondary_actions,
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
        if normalized_rest == "group" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![crate::ux::command_error(
                        MODULE_ID,
                        "proxy:group-invalid",
                        "Proxy group command is incomplete",
                        "Usage: /proxy group <name>",
                    )],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        let recognized = normalized_rest.is_empty()
            || matches!(
                normalized_rest.as_str(),
                "status" | "check" | "global" | "rule" | "profile" | "import" | "sync"
            )
            || normalized_rest.starts_with("group ")
            || normalized_rest.starts_with("profile ")
            || normalized_rest.starts_with("import ");
        if !recognized {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![crate::ux::command_error(
                        MODULE_ID,
                        "proxy:command-invalid",
                        "Unknown Proxy command",
                        "Use /proxy, /proxy status, /proxy check, /proxy group <name>, /proxy global, /proxy rule, /proxy profile, /proxy import <source>, or /proxy sync",
                    )],
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        if normalized_rest == "sync" {
            let item = if self.profiles.is_some() {
                SearchItemDto {
                    id: "proxy:profile:sync".into(),
                    module_id: MODULE_ID.into(),
                    title: "Sync convention proxy.yaml".into(),
                    subtitle: Some(
                        "Compile LumaNext/proxy.yaml into the fixed Luma Profile (not applied yet)"
                            .into(),
                    ),
                    kind: "profile_sync".into(),
                    score: 95.0,
                    primary_action_id: "sync_convention_profile".into(),
                    primary_action_label: "Sync".into(),
                    primary_action_risk: ActionRisk::Confirm,
                    primary_action_confirmation: true,
                    ..Default::default()
                }
            } else {
                profile_unavailable()
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
        if normalized_rest == "profile"
            || normalized_rest.starts_with("profile ")
            || normalized_rest == "import"
            || normalized_rest.starts_with("import ")
        {
            self.search_profiles(&normalized_rest, query.rest_raw(), query.limit, sink)
                .await;
            return;
        }
        let mut status = match await_unless_cancelled(cancel, self.core.get_status()).await {
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
        if status.profile.is_none() {
            if let Some(profiles) = &self.profiles {
                if let Ok(items) = profiles.list_profiles().await {
                    status.profile = items
                        .into_iter()
                        .find(|profile| profile.owned_by_luma && profile.current)
                        .map(|profile| profile.name);
                }
            }
        }
        self.group_keys.write().await.clear();
        self.selection_keys.write().await.clear();
        *self.last_status.write().await = Some(status.clone());
        let system = self.system_proxy.get_status().await.ok();
        if normalized_rest == "status" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: self.status_rows(&status, system.as_ref()).await,
                    removed_ids: vec![],
                })
                .await;
            return;
        }
        if normalized_rest == "check" {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: self.check_rows(&status, system.as_ref()).await,
                    removed_ids: vec![],
                })
                .await;
            return;
        }
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
                if requested_group.is_none() && group.nodes.is_empty() && group.selected.is_none() {
                    continue;
                }
                if requested_group
                    .map(|needle| !group.name.to_lowercase().contains(needle))
                    .unwrap_or(false)
                {
                    continue;
                }
                if requested_group.is_none() {
                    let group_item = Self::group_item(group, status.mode, 80.0);
                    self.group_keys
                        .write()
                        .await
                        .insert(group_item.id.clone(), group.name.clone());
                    items.push(group_item);
                } else {
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
    system_state: SystemProxyState,
    can_copy: bool,
) -> Vec<luma_protocol::ActionDescriptorDto> {
    let mut actions = vec![
        action_dto("set_global", "Set Global", ActionRisk::Confirm, true),
        action_dto("set_rule", "Set Rule", ActionRisk::Confirm, true),
        action_dto("open_proxy_status", "Details", ActionRisk::Safe, false),
    ];
    match system_state {
        SystemProxyState::On => actions.push(action_dto(
            "disable_system_proxy",
            "Disable System Proxy",
            ActionRisk::Confirm,
            true,
        )),
        SystemProxyState::Off | SystemProxyState::Mismatch => {}
        SystemProxyState::Unavailable => {}
    }
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
