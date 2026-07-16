//! Listening ports + safe process kill (`luma.ports`).
//!
//! Discover → search → preview → kill/copy/name → persist metadata → reuse.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, ClockPort, KillSignal, ListeningEndpoint, LumaModule,
    ModuleManifest, ModuleState, PasteboardPort, PortMeta, PortsMetaRepository,
    ProcessCatalogError, ProcessCatalogPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct PortsModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn ProcessCatalogPort>,
    meta: Option<Arc<dyn PortsMetaRepository>>,
    pasteboard: Arc<dyn PasteboardPort>,
    clock: Arc<dyn ClockPort>,
    endpoints: RwLock<Vec<ListeningEndpoint>>,
    meta_cache: RwLock<HashMap<u16, PortMeta>>,
    catalog_error: RwLock<Option<ProcessCatalogError>>,
    meta_error: RwLock<Option<String>>,
}

impl PortsModule {
    pub fn with_deps(
        catalog: Arc<dyn ProcessCatalogPort>,
        meta: Option<Arc<dyn PortsMetaRepository>>,
        pasteboard: Arc<dyn PasteboardPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.ports"),
                display_name: "Ports".into(),
                triggers: vec!["port".into(), "ports".into(), "kill".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("P".into()),
                    suggested_query: Some("port ".into()),
                    empty_hint: Some(
                        "port · fav · name PORT LABEL · Enter kills after confirm".into(),
                    ),
                    supports_browse: false,
                },
            },
            catalog,
            meta,
            pasteboard,
            clock,
            endpoints: RwLock::new(Vec::new()),
            meta_cache: RwLock::new(HashMap::new()),
            catalog_error: RwLock::new(None),
            meta_error: RwLock::new(None),
        }
    }

    async fn refresh_catalog(&self) {
        match self.catalog.probe() {
            Ok(()) => match self.catalog.list_listening() {
                Ok(list) => {
                    *self.catalog_error.write().await = None;
                    *self.endpoints.write().await = list;
                }
                Err(err) => {
                    *self.catalog_error.write().await = Some(err);
                    *self.endpoints.write().await = Vec::new();
                }
            },
            Err(err) => {
                *self.catalog_error.write().await = Some(err);
                *self.endpoints.write().await = Vec::new();
            }
        }
    }

    async fn refresh_meta(&self) {
        let Some(meta) = &self.meta else {
            *self.meta_cache.write().await = HashMap::new();
            *self.meta_error.write().await = None;
            return;
        };
        match meta.list() {
            Ok(rows) => {
                *self.meta_error.write().await = None;
                let mut map = HashMap::new();
                for row in rows {
                    map.insert(row.port, row);
                }
                *self.meta_cache.write().await = map;
            }
            Err(err) => {
                *self.meta_error.write().await = Some(err.to_string());
                *self.meta_cache.write().await = HashMap::new();
            }
        }
    }

    async fn refresh(&self) {
        self.refresh_catalog().await;
        self.refresh_meta().await;
        if let (Some(meta), None) = (&self.meta, self.catalog_error.read().await.as_ref()) {
            let now = self.clock.now_rfc3339().unwrap_or_default();
            if !now.is_empty() {
                for ep in self.endpoints.read().await.iter() {
                    let _ = meta.record_seen(ep.port, &now);
                }
                self.refresh_meta().await;
            }
        }
    }

    async fn emit_results(&self, sink: &SearchSink, items: Vec<SearchItem>) {
        let dtos: Vec<SearchItemDto> = items.iter().map(SearchItemDto::from).collect();
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: dtos,
                removed_ids: vec![],
            })
            .await;
    }

    fn status_row(
        id: &str,
        kind: &str,
        title: impl Into<String>,
        subtitle: Option<String>,
    ) -> SearchItem {
        SearchItem {
            id: luma_domain::ResultId::new(id),
            module_id: ModuleId::new("luma.ports"),
            title: title.into(),
            subtitle,
            kind: kind.into(),
            score: 0.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("noop"),
                label: "—".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: None,
        }
    }

    fn map_catalog_error(err: &ProcessCatalogError) -> SearchItem {
        match err {
            ProcessCatalogError::PermissionRequired {
                capability,
                guidance,
            } => Self::status_row(
                "port:permission",
                "permission_required",
                "Permission required",
                Some(format!("{capability}: {guidance}")),
            ),
            ProcessCatalogError::Unavailable(reason) => Self::status_row(
                "port:unavailable",
                "unavailable",
                "Ports unavailable",
                Some(reason.clone()),
            ),
            other => Self::status_row(
                "port:unavailable",
                "unavailable",
                "Ports unavailable",
                Some(other.to_string()),
            ),
        }
    }

    fn endpoint_id(ep: &ListeningEndpoint) -> String {
        format!("port:{}:{}:{}", ep.port, ep.pid, ep.address)
    }

    fn port_actions(favorite: bool) -> Vec<ActionDescriptor> {
        vec![
            ActionDescriptor {
                id: ActionId::new("kill"),
                label: "Kill (SIGTERM)".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            ActionDescriptor {
                id: ActionId::new("force_kill"),
                label: "Force kill (SIGKILL)".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            ActionDescriptor {
                id: ActionId::new("copy_port"),
                label: "Copy port".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy_pid"),
                label: "Copy PID".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy_command"),
                label: "Copy command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new(if favorite { "unfavorite" } else { "favorite" }),
                label: if favorite {
                    "Unfavorite".into()
                } else {
                    "Favorite".into()
                },
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("clear_name"),
                label: "Clear name".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            },
        ]
    }

    fn endpoint_item(ep: &ListeningEndpoint, meta: Option<&PortMeta>, score: f64) -> SearchItem {
        let favorite = meta.map(|m| m.favorite).unwrap_or(false);
        let name = meta
            .and_then(|m| m.display_name.clone())
            .filter(|n| !n.is_empty());
        let title = match &name {
            Some(n) => format!("{n} · :{}", ep.port),
            None => format!(":{}", ep.port),
        };
        let mut subtitle = format!("{} · pid {} · {}", ep.process_name, ep.pid, ep.address);
        if favorite {
            subtitle = format!("★ {subtitle}");
        }
        SearchItem {
            id: luma_domain::ResultId::new(Self::endpoint_id(ep)),
            module_id: ModuleId::new("luma.ports"),
            title,
            subtitle: Some(subtitle),
            kind: "port".into(),
            score,
            primary_action: ActionDescriptor {
                id: ActionId::new("kill"),
                label: "Kill (SIGTERM)".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
            secondary_actions: Self::port_actions(favorite)
                .into_iter()
                .filter(|a| a.id.as_str() != "kill")
                .collect(),
            ui_intent: None,
            action_payload: Some(serde_json::json!({
                "port": ep.port,
                "pid": ep.pid,
                "address": ep.address,
                "process_name": ep.process_name,
                "command_line": ep.command_line,
                "favorite": favorite,
            })),
        }
    }

    fn score_endpoint(ep: &ListeningEndpoint, meta: Option<&PortMeta>, needle: &str) -> f64 {
        let mut score = 40.0;
        if let Some(m) = meta {
            if m.favorite {
                score += 35.0;
            }
            if let Some(name) = &m.display_name {
                if fuzzy_match(needle, name) {
                    score += 30.0;
                }
            }
        }
        if needle.is_empty() {
            return score + (u16::MAX - ep.port) as f64 / 1000.0;
        }
        if needle.chars().all(|c| c.is_ascii_digit()) {
            if ep.port.to_string() == needle {
                score += 80.0;
            } else if ep.port.to_string().contains(needle) {
                score += 40.0;
            }
        }
        if fuzzy_match(needle, &ep.process_name) {
            score += 35.0;
        }
        if ep
            .command_line
            .as_deref()
            .is_some_and(|c| fuzzy_match(needle, c))
        {
            score += 20.0;
        }
        if fuzzy_match(needle, &ep.address) {
            score += 10.0;
        }
        if ep.pid.to_string() == needle {
            score += 50.0;
        }
        score
    }

    fn parse_name_command(rest: &str) -> Option<(u16, String)> {
        let rest = rest.trim();
        let lower = rest.to_ascii_lowercase();
        let body = lower
            .strip_prefix("name ")
            .or_else(|| lower.strip_prefix("rename "))?;
        // Use original casing from rest for the label; port from body start.
        let orig = rest
            .get(rest.len().saturating_sub(body.len())..)
            .unwrap_or(body);
        let mut parts = orig.split_whitespace();
        let port: u16 = parts.next()?.parse().ok()?;
        let label = parts.collect::<Vec<_>>().join(" ");
        if label.is_empty() || label.len() > 64 {
            return None;
        }
        if label.chars().any(|c| c.is_control()) {
            return None;
        }
        Some((port, label))
    }

    fn parse_unname_command(rest: &str) -> Option<UnnameTarget> {
        let rest = rest.trim();
        let lower = rest.to_ascii_lowercase();
        let body = lower.strip_prefix("unname ")?.trim();
        if body.is_empty() {
            return None;
        }
        if let Ok(port) = body.parse::<u16>() {
            return Some(UnnameTarget::Port(port));
        }
        Some(UnnameTarget::Name(body.to_string()))
    }

    fn payload_port_pid(item: &SearchItem) -> Option<(u16, u32)> {
        let payload = item.action_payload.as_ref()?;
        let port = payload.get("port")?.as_u64()? as u16;
        let pid = payload.get("pid")?.as_u64()? as u32;
        Some((port, pid))
    }

    async fn build_list(&self, needle: &str, favorites_only: bool) -> Vec<SearchItem> {
        let endpoints = self.endpoints.read().await.clone();
        let meta_cache = self.meta_cache.read().await.clone();
        let mut items: Vec<(f64, SearchItem)> = endpoints
            .iter()
            .filter_map(|ep| {
                let meta = meta_cache.get(&ep.port);
                if favorites_only && !meta.map(|m| m.favorite).unwrap_or(false) {
                    return None;
                }
                let score = Self::score_endpoint(ep, meta, needle);
                if !needle.is_empty() && score < 50.0 && !favorites_only {
                    // Require a meaningful match when filtering.
                    let matched = needle.chars().all(|c| c.is_ascii_digit())
                        && ep.port.to_string().contains(needle)
                        || fuzzy_match(needle, &ep.process_name)
                        || ep
                            .command_line
                            .as_deref()
                            .is_some_and(|c| fuzzy_match(needle, c))
                        || meta
                            .and_then(|m| m.display_name.as_deref())
                            .is_some_and(|n| fuzzy_match(needle, n))
                        || ep.pid.to_string() == needle;
                    if !matched {
                        return None;
                    }
                }
                Some((score, Self::endpoint_item(ep, meta, score)))
            })
            .collect();
        items.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let pa =
                        a.1.action_payload
                            .as_ref()
                            .and_then(|p| p.get("port"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    let pb =
                        b.1.action_payload
                            .as_ref()
                            .and_then(|p| p.get("port"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    pa.cmp(&pb)
                })
        });
        items.into_iter().map(|(_, item)| item).collect()
    }
}

enum UnnameTarget {
    Port(u16),
    Name(String),
}

fn fuzzy_match(needle: &str, hay: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}

fn failure_from_catalog(err: ProcessCatalogError) -> FailureKind {
    match err {
        ProcessCatalogError::PermissionRequired {
            capability,
            guidance,
        } => FailureKind::PermissionRequired {
            capability,
            guidance,
        },
        ProcessCatalogError::Unavailable(reason) => FailureKind::Unavailable {
            reason,
            retryable: true,
        },
        ProcessCatalogError::NotFound(entity) => FailureKind::NotFound { entity },
        ProcessCatalogError::InvalidInput { field, message } => {
            FailureKind::InvalidInput { field, message }
        }
        ProcessCatalogError::KillFailed { pid, reason } => FailureKind::Unavailable {
            reason: format!("kill pid {pid} failed: {reason}"),
            retryable: true,
        },
    }
}

#[async_trait]
impl LumaModule for PortsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        self.refresh().await;
        if let Some(err) = self.catalog_error.read().await.as_ref() {
            return ModuleState::Failed(err.to_string());
        }
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let rest = query.rest_raw();
        let rest_norm = query.rest_normalized();

        if rest_norm == "reload" || rest_norm.starts_with("reload ") {
            self.refresh().await;
        } else if self.catalog_error.read().await.is_some()
            || self.endpoints.read().await.is_empty()
        {
            // Refresh on ordinary searches so lists stay current.
            self.refresh_catalog().await;
            self.refresh_meta().await;
        }

        if let Some(err) = self.catalog_error.read().await.clone() {
            self.emit_results(&sink, vec![Self::map_catalog_error(&err)])
                .await;
            return;
        }

        if let Some(reason) = self.meta_error.read().await.clone() {
            self.emit_results(
                &sink,
                vec![Self::status_row(
                    "port:meta-unavailable",
                    "unavailable",
                    "Port metadata unavailable",
                    Some(reason),
                )],
            )
            .await;
            return;
        }

        if let Some((port, label)) = Self::parse_name_command(rest) {
            let item = SearchItem {
                id: luma_domain::ResultId::new(format!("port:name:{port}")),
                module_id: ModuleId::new("luma.ports"),
                title: format!("Name :{port} → {label}"),
                subtitle: Some("Save local display name".into()),
                kind: "manage".into(),
                score: 100.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("set_name"),
                    label: "Save name".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: Some(serde_json::json!({ "port": port, "name": label })),
            };
            self.emit_results(&sink, vec![item]).await;
            return;
        }

        if rest_norm.starts_with("name ") || rest_norm.starts_with("rename ") {
            self.emit_results(
                &sink,
                vec![Self::status_row(
                    "port:name-usage",
                    "status",
                    "Usage: port name 3000 api",
                    Some("Port must be 1–65535; label up to 64 chars".into()),
                )],
            )
            .await;
            return;
        }

        if let Some(target) = Self::parse_unname_command(rest) {
            let (title, payload) = match target {
                UnnameTarget::Port(port) => (
                    format!("Clear name for :{port}"),
                    serde_json::json!({ "port": port }),
                ),
                UnnameTarget::Name(name) => {
                    let port = self
                        .meta_cache
                        .read()
                        .await
                        .iter()
                        .find(|(_, m)| {
                            m.display_name
                                .as_deref()
                                .is_some_and(|n| n.eq_ignore_ascii_case(&name))
                        })
                        .map(|(p, _)| *p);
                    match port {
                        Some(port) => (
                            format!("Clear name '{name}' (:{port})"),
                            serde_json::json!({ "port": port }),
                        ),
                        None => {
                            self.emit_results(
                                &sink,
                                vec![Self::status_row(
                                    "port:unname-missing",
                                    "status",
                                    format!("No named port matching '{name}'"),
                                    None,
                                )],
                            )
                            .await;
                            return;
                        }
                    }
                }
            };
            let item = SearchItem {
                id: luma_domain::ResultId::new("port:unname"),
                module_id: ModuleId::new("luma.ports"),
                title,
                subtitle: Some("Remove local display name".into()),
                kind: "manage".into(),
                score: 100.0,
                primary_action: ActionDescriptor {
                    id: ActionId::new("clear_name"),
                    label: "Clear name".into(),
                    risk: ActionRisk::Confirm,
                    confirmation: true,
                },
                secondary_actions: vec![],
                ui_intent: None,
                action_payload: Some(payload),
            };
            self.emit_results(&sink, vec![item]).await;
            return;
        }

        let favorites_only = rest_norm == "fav" || rest_norm == "favorites";
        let needle = if favorites_only {
            ""
        } else {
            rest_norm.as_str()
        };
        let mut items = self.build_list(needle, favorites_only).await;
        if items.is_empty() {
            let title = if favorites_only {
                "No favorite ports"
            } else if needle.is_empty() {
                "No listening TCP ports"
            } else {
                "No matching ports"
            };
            items.push(Self::status_row(
                "port:empty",
                "status",
                title,
                Some("Try port reload · or start a local server".into()),
            ));
        }
        if cancel.is_cancelled() {
            return;
        }
        self.emit_results(&sink, items).await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind == "port" {
            let favorite = result
                .action_payload
                .as_ref()
                .and_then(|p| p.get("favorite"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            return Self::port_actions(favorite);
        }
        if result.kind == "manage" {
            return vec![result.primary_action.clone()];
        }
        vec![ActionDescriptor {
            id: ActionId::new("noop"),
            label: "—".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind != "port" {
            return result
                .subtitle
                .clone()
                .or_else(|| Some(result.title.clone()));
        }
        let payload = result.action_payload.as_ref()?;
        let port = payload.get("port")?.as_u64()?;
        let pid = payload.get("pid")?.as_u64()?;
        let address = payload.get("address")?.as_str().unwrap_or("?");
        let process = payload.get("process_name")?.as_str().unwrap_or("?");
        let command = payload
            .get("command_line")
            .and_then(|v| v.as_str())
            .unwrap_or(process);
        let meta = self.meta_cache.read().await.get(&(port as u16)).cloned();
        let mut lines = vec![
            format!("Port      {port}"),
            format!("Address   {address}"),
            format!("PID       {pid}"),
            format!("Process   {process}"),
            format!("Command   {command}"),
        ];
        if let Some(m) = meta {
            if let Some(name) = m.display_name {
                lines.push(format!("Name      {name}"));
            }
            lines.push(format!(
                "Favorite  {}",
                if m.favorite { "yes" } else { "no" }
            ));
            if let Some(seen) = m.last_seen_at {
                lines.push(format!("Last seen {seen}"));
            }
            if m.kill_count > 0 {
                lines.push(format!("Kills     {}", m.kill_count));
            }
        }
        lines.push(String::new());
        lines.push("Enter confirms SIGTERM. Force kill is in the action picker.".into());
        Some(lines.join("\n"))
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        let id = action.action.id.as_str();
        if id == "noop" {
            return ActionOutcome::Success { message: None };
        }

        if matches!(id, "kill" | "force_kill") {
            if action.action.confirmation && !action.confirmation {
                return ActionOutcome::Failed {
                    kind: FailureKind::SecurityDenied {
                        reason: "confirmation required to kill process".into(),
                    },
                };
            }
            let Some((port, pid)) = Self::payload_port_pid(&action.result) else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "payload".into(),
                        message: "missing port/pid".into(),
                    },
                };
            };
            let signal = if id == "force_kill" {
                KillSignal::Kill
            } else {
                KillSignal::Term
            };
            let catalog = self.catalog.clone();
            let outcome =
                await_unless_cancelled(&cancel, async move { catalog.kill(pid, signal) }).await;
            let Some(result) = outcome else {
                return ActionOutcome::Cancelled;
            };
            return match result {
                Ok(()) => {
                    if let Some(meta) = &self.meta {
                        if let Ok(now) = self.clock.now_rfc3339() {
                            let _ = meta.record_kill(port, &now);
                        }
                    }
                    self.refresh().await;
                    let label = if signal == KillSignal::Kill {
                        "SIGKILL"
                    } else {
                        "SIGTERM"
                    };
                    ActionOutcome::Success {
                        message: Some(format!("Sent {label} to pid {pid} (:{port})")),
                    }
                }
                Err(err) => ActionOutcome::Failed {
                    kind: failure_from_catalog(err),
                },
            };
        }

        if id == "copy_port" || id == "copy_pid" || id == "copy_command" {
            let payload = action.result.action_payload.as_ref();
            let text = match id {
                "copy_port" => payload
                    .and_then(|p| p.get("port"))
                    .and_then(|v| v.as_u64())
                    .map(|p| p.to_string()),
                "copy_pid" => payload
                    .and_then(|p| p.get("pid"))
                    .and_then(|v| v.as_u64())
                    .map(|p| p.to_string()),
                _ => payload
                    .and_then(|p| p.get("command_line"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        payload
                            .and_then(|p| p.get("process_name"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    }),
            };
            let Some(text) = text else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "payload".into(),
                        message: "nothing to copy".into(),
                    },
                };
            };
            let pasteboard = self.pasteboard.clone();
            let copied = await_unless_cancelled(&cancel, async move {
                pasteboard.write_text(&text).await.map(|_| text)
            })
            .await;
            return match copied {
                None => ActionOutcome::Cancelled,
                Some(Ok(text)) => ActionOutcome::Success {
                    message: Some(format!("Copied {text}")),
                },
                Some(Err(err)) => ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                },
            };
        }

        if id == "favorite" || id == "unfavorite" {
            let Some(meta) = &self.meta else {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: "port metadata store unavailable".into(),
                        retryable: false,
                    },
                };
            };
            let Some((port, _)) = Self::payload_port_pid(&action.result) else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "payload".into(),
                        message: "missing port".into(),
                    },
                };
            };
            let favorite = id == "favorite";
            return match meta.set_favorite(port, favorite) {
                Ok(()) => {
                    self.refresh_meta().await;
                    ActionOutcome::Success {
                        message: Some(if favorite {
                            format!("Favorited :{port}")
                        } else {
                            format!("Unfavorited :{port}")
                        }),
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

        if id == "set_name" {
            let Some(meta) = &self.meta else {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: "port metadata store unavailable".into(),
                        retryable: false,
                    },
                };
            };
            let payload = action.result.action_payload.as_ref();
            let port = payload
                .and_then(|p| p.get("port"))
                .and_then(|v| v.as_u64())
                .map(|p| p as u16);
            let name = payload
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let (Some(port), Some(name)) = (port, name) else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "payload".into(),
                        message: "name requires port and label".into(),
                    },
                };
            };
            return match meta.set_display_name(port, Some(&name)) {
                Ok(()) => {
                    self.refresh_meta().await;
                    ActionOutcome::Success {
                        message: Some(format!("Named :{port} → {name}")),
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

        if id == "clear_name" {
            if action.action.confirmation && !action.confirmation {
                return ActionOutcome::Failed {
                    kind: FailureKind::SecurityDenied {
                        reason: "confirmation required to clear name".into(),
                    },
                };
            }
            let Some(meta) = &self.meta else {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: "port metadata store unavailable".into(),
                        retryable: false,
                    },
                };
            };
            let port = action
                .result
                .action_payload
                .as_ref()
                .and_then(|p| p.get("port"))
                .and_then(|v| v.as_u64())
                .map(|p| p as u16);
            let Some(port) = port else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "payload".into(),
                        message: "missing port".into(),
                    },
                };
            };
            return match meta.set_display_name(port, None) {
                Ok(()) => {
                    self.refresh_meta().await;
                    ActionOutcome::Success {
                        message: Some(format!("Cleared name for :{port}")),
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

        ActionOutcome::Failed {
            kind: FailureKind::NotFound {
                entity: format!("action {id}"),
            },
        }
    }

    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakePasteboard, FakeProcessCatalog, FixedClock, MemoryPortsMetaRepository,
    };
    use tokio::sync::mpsc;

    fn sample_endpoints() -> Vec<ListeningEndpoint> {
        vec![
            ListeningEndpoint {
                port: 3000,
                address: "*".into(),
                protocol: "tcp".into(),
                pid: 4242,
                process_name: "node".into(),
                command_line: Some("node server.js".into()),
                user: Some("me".into()),
            },
            ListeningEndpoint {
                port: 8080,
                address: "127.0.0.1".into(),
                protocol: "tcp".into(),
                pid: 5151,
                process_name: "Python".into(),
                command_line: Some("python -m http.server".into()),
                user: Some("me".into()),
            },
            ListeningEndpoint {
                port: 5432,
                address: "127.0.0.1".into(),
                protocol: "tcp".into(),
                pid: 1001,
                process_name: "postgres".into(),
                command_line: Some("postgres".into()),
                user: Some("me".into()),
            },
        ]
    }

    fn module_with(catalog: FakeProcessCatalog, meta: MemoryPortsMetaRepository) -> PortsModule {
        PortsModule::with_deps(
            Arc::new(catalog),
            Some(Arc::new(meta)),
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock::new("2026-07-16", "2026-07-16T10:00:00Z")),
        )
    }

    fn warmup_ctx() -> WarmupContext {
        WarmupContext {
            cancel: CancellationToken::new(),
        }
    }

    async fn collect_search(module: &PortsModule, raw: &str) -> Vec<SearchItemDto> {
        let (tx, mut rx) = mpsc::channel(8);
        let q = Query::parse_with_prefixes(raw, 50, |t| matches!(t, "port" | "ports" | "kill"));
        module.search(q, tx, CancellationToken::new()).await;
        match rx.recv().await {
            Some(Event::ResultsChunk { upserts, .. }) => upserts,
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn lists_and_filters_by_port_and_process() {
        let module = module_with(
            FakeProcessCatalog::with_endpoints(sample_endpoints()),
            MemoryPortsMetaRepository::new(),
        );
        module.warmup(warmup_ctx()).await;
        let all = collect_search(&module, "port ").await;
        assert!(all.iter().any(|r| r.title.contains(":3000")));
        assert!(all.iter().any(|r| r.title.contains(":8080")));

        let by_port = collect_search(&module, "port 3000").await;
        assert_eq!(by_port.len(), 1);
        assert!(by_port[0].title.contains(":3000"));

        let by_proc = collect_search(&module, "port python").await;
        assert_eq!(by_proc.len(), 1);
        assert!(by_proc[0].subtitle.as_deref().unwrap().contains("Python"));
    }

    #[tokio::test]
    async fn empty_when_no_listeners() {
        let module = module_with(FakeProcessCatalog::new(), MemoryPortsMetaRepository::new());
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port ").await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "status");
        assert!(rows[0].title.contains("No listening"));
    }

    #[tokio::test]
    async fn unavailable_when_tool_missing() {
        let catalog = FakeProcessCatalog::new();
        *catalog.probe_error.lock().unwrap() =
            Some(ProcessCatalogError::Unavailable("lsof not found".into()));
        let module = module_with(catalog, MemoryPortsMetaRepository::new());
        let rows = collect_search(&module, "port ").await;
        assert_eq!(rows[0].kind, "unavailable");
    }

    #[tokio::test]
    async fn permission_row_from_list_error() {
        let catalog = FakeProcessCatalog::new();
        *catalog.list_error.lock().unwrap() = Some(ProcessCatalogError::PermissionRequired {
            capability: "process_list".into(),
            guidance: "Grant Full Disk Access".into(),
        });
        let module = module_with(catalog, MemoryPortsMetaRepository::new());
        let rows = collect_search(&module, "port ").await;
        assert_eq!(rows[0].kind, "permission_required");
    }

    #[tokio::test]
    async fn kill_requires_confirmation_and_updates_catalog() {
        let catalog = FakeProcessCatalog::with_endpoints(sample_endpoints());
        let meta = MemoryPortsMetaRepository::new();
        let module = module_with(catalog, meta);
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port 3000").await;
        let item = rows[0].clone().into_domain();
        let denied = module
            .perform(
                ActionRequest {
                    result: item.clone(),
                    action: ActionDescriptor {
                        id: ActionId::new("kill"),
                        label: "Kill".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));

        let ok = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("kill"),
                        label: "Kill".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(ok, ActionOutcome::Success { .. }));
        let after = collect_search(&module, "port 3000").await;
        assert_eq!(after[0].kind, "status");
    }

    #[tokio::test]
    async fn cancel_during_kill_returns_cancelled() {
        let catalog = FakeProcessCatalog::with_endpoints(sample_endpoints());
        let module = module_with(catalog, MemoryPortsMetaRepository::new());
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port 3000").await;
        let item = rows[0].clone().into_domain();
        let token = CancellationToken::new();
        token.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("kill"),
                        label: "Kill".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                token,
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Cancelled));
    }

    #[tokio::test]
    async fn name_favorite_persist_and_filter() {
        let module = module_with(
            FakeProcessCatalog::with_endpoints(sample_endpoints()),
            MemoryPortsMetaRepository::new(),
        );
        module.warmup(warmup_ctx()).await;
        let name_rows = collect_search(&module, "port name 3000 api").await;
        assert_eq!(name_rows[0].kind, "manage");
        let item = name_rows[0].clone().into_domain();
        let named = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("set_name"),
                        label: "Save".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(named, ActionOutcome::Success { .. }));

        let listed = collect_search(&module, "port api").await;
        assert!(listed[0].title.starts_with("api"));

        let port_rows = collect_search(&module, "port 3000").await;
        let port_item = port_rows[0].clone().into_domain();
        let _ = module
            .perform(
                ActionRequest {
                    result: port_item,
                    action: ActionDescriptor {
                        id: ActionId::new("favorite"),
                        label: "Favorite".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        let favs = collect_search(&module, "port fav").await;
        assert_eq!(favs.len(), 1);
        assert!(favs[0].title.contains("api"));
    }

    #[tokio::test]
    async fn meta_read_failure_surfaces_unavailable() {
        let meta = MemoryPortsMetaRepository::new();
        meta.set_list_error(Some("disk full".into()));
        let module = module_with(FakeProcessCatalog::with_endpoints(sample_endpoints()), meta);
        let rows = collect_search(&module, "port ").await;
        assert_eq!(rows[0].kind, "unavailable");
        assert!(rows[0].title.contains("metadata"));
    }

    #[tokio::test]
    async fn kill_failure_restores_honest_state() {
        let catalog = FakeProcessCatalog::with_endpoints(sample_endpoints());
        *catalog.kill_error.lock().unwrap() = Some(ProcessCatalogError::KillFailed {
            pid: 4242,
            reason: "busy".into(),
        });
        let module = module_with(catalog, MemoryPortsMetaRepository::new());
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port 3000").await;
        let item = rows[0].clone().into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("kill"),
                        label: "Kill".into(),
                        risk: ActionRisk::Destructive,
                        confirmation: true,
                    },
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Failed { .. }));
        let still = collect_search(&module, "port 3000").await;
        assert_eq!(still[0].kind, "port");
    }

    #[tokio::test]
    async fn invalid_name_command_shows_usage() {
        let module = module_with(
            FakeProcessCatalog::with_endpoints(sample_endpoints()),
            MemoryPortsMetaRepository::new(),
        );
        let rows = collect_search(&module, "port name notaport label").await;
        assert_eq!(rows[0].kind, "status");
        assert!(rows[0].title.contains("Usage"));
    }

    #[tokio::test]
    async fn preview_includes_process_details() {
        let module = module_with(
            FakeProcessCatalog::with_endpoints(sample_endpoints()),
            MemoryPortsMetaRepository::new(),
        );
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port 8080").await;
        let item = rows[0].clone().into_domain();
        let preview = module.preview(&item).await.unwrap();
        assert!(preview.contains("Python"));
        assert!(preview.contains("8080"));
        assert!(preview.contains("SIGTERM"));
    }

    #[tokio::test]
    async fn sorting_prefers_favorites_and_exact_port() {
        let meta = MemoryPortsMetaRepository::new();
        meta.set_favorite(8080, true).unwrap();
        let module = module_with(FakeProcessCatalog::with_endpoints(sample_endpoints()), meta);
        module.warmup(warmup_ctx()).await;
        let rows = collect_search(&module, "port ").await;
        assert!(
            rows[0].title.contains(":8080") || rows[0].subtitle.as_deref().unwrap().contains('★')
        );
        let exact = collect_search(&module, "port 5432").await;
        assert_eq!(exact[0].title, ":5432");
    }
}
