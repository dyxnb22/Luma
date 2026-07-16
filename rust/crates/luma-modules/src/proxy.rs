//! Mihomo controller-first proxy module for local macOS use.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    ProfileSource, ProfileStoreError, ProfileStorePort, ProfileSummary, ProxyCoreError,
    ProxyCorePort, ProxyGroup, ProxyMode, ProxyNode, ProxyStatus, SearchMode, SearchSink,
    SystemProxyError, SystemProxyPort, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.proxy";

pub struct ProxyModule {
    manifest: ModuleManifest,
    core: Arc<dyn ProxyCorePort>,
    system_proxy: Arc<dyn SystemProxyPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    profiles: Option<Arc<dyn ProfileStorePort>>,
    last_status: RwLock<Option<ProxyStatus>>,
    selection_keys: RwLock<HashMap<String, (String, String)>>,
    import_keys: RwLock<HashMap<String, ImportIntent>>,
}

#[derive(Clone)]
enum ImportIntent {
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
                    suggested_query: Some("proxy ".into()),
                    empty_hint: Some("proxy · inspect Mihomo and system proxy".into()),
                    supports_browse: true,
                },
            },
            core,
            system_proxy,
            pasteboard,
            profiles: None,
            last_status: RwLock::new(None),
            selection_keys: RwLock::new(HashMap::new()),
            import_keys: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_profile_store(mut self, profiles: Arc<dyn ProfileStorePort>) -> Self {
        self.profiles = Some(profiles);
        self
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

    async fn search_profiles(
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
                    subtitle: Some("Use proxy import <HTTPS URL or local YAML path>".into()),
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

fn profile_item(profile: ProfileSummary) -> SearchItemDto {
    let updated = profile
        .updated_at
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".into());
    let primary = if profile.owned_by_luma {
        "use_profile"
    } else {
        "noop"
    };
    let details = if !profile.metadata_available {
        format!(
            "metadata unavailable · updated {} · {}{}",
            updated,
            profile.source.label(),
            if profile.current { " · current" } else { "" }
        )
    } else {
        format!(
            "{} nodes · {} groups · {} rules · updated {} · {}{}",
            profile.node_count,
            profile.group_count,
            profile.rule_count,
            updated,
            profile.source.label(),
            if profile.current { " · current" } else { "" }
        )
    };
    let owned_by_luma = profile.owned_by_luma;
    let external_uid_in_name = !owned_by_luma
        && profile
            .name
            .to_lowercase()
            .contains(&profile.id.to_lowercase());
    let title = if external_uid_in_name {
        // Clash Verge's UID is an internal identifier. Do not let a user-controlled display name
        // smuggle it back into UI when this item is deliberately read-only.
        "Clash Verge Profile".into()
    } else {
        redact_label(&profile.name)
    };
    let mut secondary_actions = Vec::new();
    if owned_by_luma {
        secondary_actions.push(action_dto(
            "delete_profile",
            "Delete",
            ActionRisk::Confirm,
            true,
        ));
        if profile.source == ProfileSource::Subscription {
            secondary_actions.push(action_dto(
                "refresh_profile",
                "Refresh",
                ActionRisk::Safe,
                false,
            ));
        }
    }
    SearchItemDto {
        id: if owned_by_luma {
            format!("proxy:profile:{}", profile.id)
        } else {
            format!("proxy:profile:readonly:{}", opaque_component(&profile.id))
        },
        module_id: MODULE_ID.into(),
        title,
        subtitle: Some(details),
        kind: "profile".into(),
        score: if profile.current { 100.0 } else { 85.0 },
        primary_action_id: primary.into(),
        primary_action_label: if owned_by_luma {
            "Use".into()
        } else {
            "Read-only".into()
        },
        primary_action_risk: if owned_by_luma {
            ActionRisk::Confirm
        } else {
            ActionRisk::Safe
        },
        primary_action_confirmation: owned_by_luma,
        secondary_actions,
        action_payload: owned_by_luma.then(|| serde_json::json!({"profile_id": profile.id})),
        ..Default::default()
    }
}

fn profile_unavailable() -> SearchItemDto {
    SearchItemDto {
        id: "proxy:profile:unavailable".into(),
        module_id: MODULE_ID.into(),
        title: "Profiles unavailable".into(),
        subtitle: Some("Luma Profile storage is not configured".into()),
        kind: "unavailable".into(),
        primary_action_id: "refresh".into(),
        primary_action_label: "Refresh".into(),
        ..Default::default()
    }
}
fn profile_error_item(error: &ProfileStoreError) -> SearchItemDto {
    SearchItemDto {
        id: "proxy:profile:unavailable".into(),
        module_id: MODULE_ID.into(),
        title: "Profiles unavailable".into(),
        subtitle: Some(profile_error_message(error)),
        kind: "unavailable".into(),
        primary_action_id: "refresh".into(),
        primary_action_label: "Refresh".into(),
        ..Default::default()
    }
}
fn profile_error_message(error: &ProfileStoreError) -> String {
    error.to_string()
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
        // Controller paths can include provider, group, or node labels. Keep adapter details out
        // of ActionOutcome, which is rendered and may be retained by callers.
        ProxyCoreError::Timeout(_) => FailureKind::Timeout {
            operation: "Mihomo controller request".into(),
        },
        ProxyCoreError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProxyCoreError::NotFound(_) => FailureKind::NotFound {
            entity: "Proxy item".into(),
        },
        ProxyCoreError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        ProxyCoreError::NotConfigured(remediation) => FailureKind::NotConfigured { remediation },
        ProxyCoreError::Unavailable(_) => FailureKind::Unavailable {
            reason: "Mihomo controller is unavailable".into(),
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

fn profile_failure(error: ProfileStoreError) -> FailureKind {
    match error {
        ProfileStoreError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProfileStoreError::NotFound(entity) => FailureKind::NotFound { entity },
        ProfileStoreError::Timeout => FailureKind::Timeout {
            operation: "profile operation".into(),
        },
        ProfileStoreError::SecurityDenied(reason) => FailureKind::SecurityDenied { reason },
        ProfileStoreError::Conflict(reason) => FailureKind::Conflict { reason },
        ProfileStoreError::NotConfigured(remediation) => FailureKind::NotConfigured { remediation },
        ProfileStoreError::Unsupported(reason) => FailureKind::Unavailable {
            reason,
            retryable: false,
        },
        ProfileStoreError::Unavailable(reason) => FailureKind::Unavailable {
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
            "profile_import_help" => vec![],
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
                    .enable(
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
        self.selection_keys.write().await.clear();
        self.import_keys.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakePasteboard, FakeProxyCore, FakeSystemProxy, ProfileImportResult, ProfileSource,
        ProfileStoreError, ProfileStorePort, ProxyPorts, SystemProxySetting, SystemProxyStatus,
    };
    use luma_test_support::collect_search_items;
    use std::path::Path;

    fn module() -> (ProxyModule, Arc<FakeProxyCore>, Arc<FakeSystemProxy>) {
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
        let module = ProxyModule::with_deps(
            core.clone(),
            system.clone(),
            Arc::new(FakePasteboard::new()),
        );
        (module, core, system)
    }

    #[tokio::test]
    async fn search_redacts_uuid_and_shows_status_and_selected_node() {
        let (module, _, _) = module();
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
        let (module, core, _) = module();
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
            .find(|item| item.kind == "status")
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
            vec![(Some(7897), Some(7897))]
        );
    }
}
