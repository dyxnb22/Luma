use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    format_connection_subtitle, sanitize_identity_display, sftp_args, ssh_connect_args,
    ActionOutcome, ActionRequest, ClockPort, LumaModule, ModuleManifest, ModuleState,
    PasteboardPort, ResolvedSshHost, SearchMode, SearchSink, SshConfigPort, SshConfigState,
    SshHostMeta, SshMetaRepository, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct SshModule {
    manifest: ModuleManifest,
    config: Arc<dyn SshConfigPort>,
    meta: Option<Arc<dyn SshMetaRepository>>,
    pasteboard: Arc<dyn PasteboardPort>,
    clock: Arc<dyn ClockPort>,
    aliases: RwLock<Vec<String>>,
    resolved_cache: RwLock<HashMap<String, ResolvedSshHost>>,
    meta_cache: RwLock<HashMap<String, SshHostMeta>>,
    meta_error: RwLock<Option<String>>,
}

impl SshModule {
    pub fn with_deps(
        config: Arc<dyn SshConfigPort>,
        meta: Option<Arc<dyn SshMetaRepository>>,
        pasteboard: Arc<dyn PasteboardPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.ssh"),
                display_name: "SSH".into(),
                triggers: vec!["ssh".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("S".into()),
                    suggested_query: Some("ssh ".into()),
                    empty_hint: Some(
                        "ssh · fav · recent · rename ALIAS NAME · Enter to connect".into(),
                    ),
                    supports_browse: false,
                },
            },
            config,
            meta,
            pasteboard,
            clock,
            aliases: RwLock::new(Vec::new()),
            resolved_cache: RwLock::new(HashMap::new()),
            meta_cache: RwLock::new(HashMap::new()),
            meta_error: RwLock::new(None),
        }
    }

    async fn refresh(&self) {
        match self.config.list_aliases() {
            Ok(aliases) => {
                *self.aliases.write().await = aliases;
            }
            Err(_) => {
                *self.aliases.write().await = Vec::new();
            }
        }
        if let Some(meta) = &self.meta {
            match meta.list() {
                Ok(rows) => {
                    *self.meta_error.write().await = None;
                    let mut map = HashMap::new();
                    for row in rows {
                        map.insert(row.alias.clone(), row);
                    }
                    *self.meta_cache.write().await = map;
                }
                Err(err) => {
                    *self.meta_error.write().await = Some(err.to_string());
                    *self.meta_cache.write().await = HashMap::new();
                }
            }
        }
    }

    async fn resolve_host(&self, alias: &str) -> Option<ResolvedSshHost> {
        if let Some(cached) = self.resolved_cache.read().await.get(alias).cloned() {
            return Some(cached);
        }
        match self.config.resolve(alias) {
            Ok(host) => {
                self.resolved_cache
                    .write()
                    .await
                    .insert(alias.to_string(), host.clone());
                Some(host)
            }
            Err(_) => None,
        }
    }

    fn alias_from_item(item: &SearchItem) -> Option<String> {
        if let Some(payload) = &item.action_payload {
            if let Some(alias) = payload.get("alias").and_then(|v| v.as_str()) {
                return Some(alias.to_string());
            }
        }
        item.id
            .as_str()
            .strip_prefix("ssh:")
            .map(str::to_string)
            .filter(|id| {
                !id.contains(':')
                    && !matches!(
                        id.as_str(),
                        "reload" | "hint" | "empty" | "unavailable" | "not-configured"
                    )
            })
    }

    fn host_row_id(alias: &str) -> String {
        format!("ssh:{alias}")
    }

    fn fuzzy_match(needle: &str, hay: &str) -> bool {
        hay.to_lowercase().contains(&needle.to_lowercase())
    }

    fn score_host(
        alias: &str,
        host: &ResolvedSshHost,
        meta: Option<&SshHostMeta>,
        needle: &str,
    ) -> f64 {
        let mut score = 50.0;
        if let Some(m) = meta {
            if m.favorite {
                score += 40.0;
            }
            if let Some(ts) = &m.last_connected_at {
                score += 5.0;
                let _ = ts;
            }
            if let Some(name) = &m.display_name {
                if Self::fuzzy_match(needle, name) {
                    score += 25.0;
                }
            }
        }
        if alias.eq_ignore_ascii_case(needle) {
            score += 50.0;
        } else if Self::fuzzy_match(needle, alias) {
            score += 30.0;
        }
        if host
            .hostname
            .as_deref()
            .is_some_and(|h| Self::fuzzy_match(needle, h))
        {
            score += 20.0;
        }
        if host
            .user
            .as_deref()
            .is_some_and(|u| Self::fuzzy_match(needle, u))
        {
            score += 15.0;
        }
        score
    }

    fn compare_last_connected(
        a: Option<&SshHostMeta>,
        b: Option<&SshHostMeta>,
    ) -> std::cmp::Ordering {
        let ts_a = a.and_then(|m| m.last_connected_at.as_deref()).unwrap_or("");
        let ts_b = b.and_then(|m| m.last_connected_at.as_deref()).unwrap_or("");
        ts_b.cmp(ts_a)
    }

    fn host_actions(favorite: bool) -> Vec<ActionDescriptor> {
        vec![
            ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("sftp"),
                label: "Open SFTP".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy_alias"),
                label: "Copy alias".into(),
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
                id: ActionId::new("delete_metadata"),
                label: "Delete local metadata".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            },
        ]
    }

    async fn alias_is_known(&self, alias: &str) -> bool {
        let cached = self.aliases.read().await;
        if !cached.is_empty() {
            return cached.iter().any(|a| a == alias);
        }
        drop(cached);
        self.config
            .list_aliases()
            .ok()
            .is_some_and(|aliases| aliases.iter().any(|a| a == alias))
    }

    fn meta_unavailable_row(reason: String) -> SearchItem {
        Self::status_row(
            "ssh:meta-unavailable",
            "unavailable",
            "SSH metadata unavailable",
            Some(reason),
        )
    }

    async fn favorite_for_alias(&self, alias: &str) -> bool {
        self.meta_cache
            .read()
            .await
            .get(alias)
            .map(|m| m.favorite)
            .unwrap_or(false)
    }

    async fn emit_results(&self, sink: &SearchSink, items: Vec<SearchItem>, removed: Vec<String>) {
        let dtos: Vec<SearchItemDto> = items.iter().map(SearchItemDto::from).collect();
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: dtos,
                removed_ids: removed,
            })
            .await;
    }

    fn status_row(id: &str, kind: &str, title: &str, subtitle: Option<String>) -> SearchItem {
        SearchItem {
            id: luma_domain::ResultId::new(id),
            module_id: ModuleId::new("luma.ssh"),
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

    fn host_item(
        alias: &str,
        host: &ResolvedSshHost,
        meta: Option<&SshHostMeta>,
        score: f64,
    ) -> SearchItem {
        let title = meta
            .and_then(|m| m.display_name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| alias.to_string());
        let favorite = meta.map(|m| m.favorite).unwrap_or(false);
        let mut subtitle = format_connection_subtitle(host);
        if favorite {
            subtitle.push_str(" · ★");
        }
        SearchItem {
            id: luma_domain::ResultId::new(Self::host_row_id(alias)),
            module_id: ModuleId::new("luma.ssh"),
            title,
            subtitle: Some(subtitle),
            kind: "ssh_host".into(),
            score,
            primary_action: ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: Self::host_actions(favorite),
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": alias })),
        }
    }
}

struct RenamePayload {
    alias: String,
    display_name: String,
}

fn parse_rename_query(rest: &str) -> Option<RenamePayload> {
    let rest = rest.trim();
    if rest.len() < 7 || !rest[..7].eq_ignore_ascii_case("rename ") {
        return None;
    }
    let after_prefix = rest[7..].trim_start();
    let mut parts = after_prefix.splitn(2, char::is_whitespace);
    let alias = parts.next()?.trim();
    let display_name = parts.next()?.trim();
    if alias.is_empty() || display_name.is_empty() {
        return None;
    }
    Some(RenamePayload {
        alias: alias.to_string(),
        display_name: display_name.to_string(),
    })
}

#[async_trait]
impl LumaModule for SshModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if await_unless_cancelled(&ctx.cancel, self.refresh())
            .await
            .is_none()
        {
            return ModuleState::Cold;
        }
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        self.refresh().await;

        match self.config.config_state() {
            SshConfigState::NotConfigured => {
                self.emit_results(
                    &sink,
                    vec![Self::status_row(
                        "ssh:not-configured",
                        "not_configured",
                        "SSH config not found",
                        Some("Create ~/.ssh/config with Host entries".into()),
                    )],
                    vec![],
                )
                .await;
                return;
            }
            SshConfigState::Unavailable(reason) => {
                self.emit_results(
                    &sink,
                    vec![Self::status_row(
                        "ssh:unavailable",
                        "unavailable",
                        "SSH config unavailable",
                        Some(reason),
                    )],
                    vec![],
                )
                .await;
                return;
            }
            SshConfigState::Found => {}
        }

        if !self.config.ssh_available() {
            self.emit_results(
                &sink,
                vec![Self::status_row(
                    "ssh:unavailable",
                    "unavailable",
                    "OpenSSH ssh command unavailable",
                    Some("install OpenSSH client".into()),
                )],
                vec![],
            )
            .await;
            return;
        }

        let aliases = self.aliases.read().await.clone();
        let meta_map = self.meta_cache.read().await.clone();
        let meta_error = self.meta_error.read().await.clone();
        let rest = query.rest_raw().trim();
        let rest_check = query.rest_normalized();

        if rest_check == "reload" || rest_check == "refresh" {
            self.emit_results(
                &sink,
                vec![SearchItem {
                    id: luma_domain::ResultId::new("ssh:reload"),
                    module_id: ModuleId::new("luma.ssh"),
                    title: "Reload SSH config cache".into(),
                    subtitle: Some("re-read ~/.ssh/config and ssh -G results".into()),
                    kind: "status".into(),
                    score: 90.0,
                    primary_action: ActionDescriptor {
                        id: ActionId::new("reload_config"),
                        label: "Reload".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    secondary_actions: vec![],
                    ui_intent: None,
                    action_payload: None,
                }],
                vec![],
            )
            .await;
            return;
        }

        if let Some(payload) = parse_rename_query(rest) {
            self.emit_results(
                &sink,
                vec![SearchItem {
                    id: luma_domain::ResultId::new(format!("ssh:rename:{}", payload.alias)),
                    module_id: ModuleId::new("luma.ssh"),
                    title: format!("Rename {}", payload.alias),
                    subtitle: Some(payload.display_name.clone()),
                    kind: "update".into(),
                    score: 95.0,
                    primary_action: ActionDescriptor {
                        id: ActionId::new("rename"),
                        label: "Save display name".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    secondary_actions: vec![],
                    ui_intent: None,
                    action_payload: Some(serde_json::json!({
                        "alias": payload.alias,
                        "display_name": payload.display_name,
                    })),
                }],
                vec![],
            )
            .await;
            return;
        }

        let favorites_only = matches!(
            rest_check.as_str(),
            "fav" | "favs" | "favorite" | "favorites"
        );
        let recent_only = rest_check == "recent";
        let needle = if favorites_only || recent_only {
            String::new()
        } else {
            rest_check.clone()
        };

        let mut rows: Vec<(f64, String)> = Vec::new();
        for alias in &aliases {
            let host = match self.resolve_host(alias).await {
                Some(h) => h,
                None => continue,
            };
            let meta = meta_map.get(alias);
            if favorites_only && !meta.is_some_and(|m| m.favorite) {
                continue;
            }
            if recent_only && meta.and_then(|m| m.last_connected_at.as_ref()).is_none() {
                continue;
            }
            if !needle.is_empty() {
                let matches = alias.to_lowercase().contains(&needle)
                    || meta
                        .and_then(|m| m.display_name.as_deref())
                        .is_some_and(|n| n.to_lowercase().contains(&needle))
                    || host
                        .hostname
                        .as_deref()
                        .is_some_and(|h| h.to_lowercase().contains(&needle))
                    || host
                        .user
                        .as_deref()
                        .is_some_and(|u| u.to_lowercase().contains(&needle));
                if !matches {
                    continue;
                }
            }
            let score = Self::score_host(alias, &host, meta, &needle);
            rows.push((score, alias.clone()));
        }

        rows.sort_by(|a, b| {
            let meta_a = meta_map.get(&a.1);
            let meta_b = meta_map.get(&b.1);
            let fav_a = meta_a.is_some_and(|m| m.favorite);
            let fav_b = meta_b.is_some_and(|m| m.favorite);
            fav_b
                .cmp(&fav_a)
                .then_with(|| Self::compare_last_connected(meta_b, meta_a))
                .then_with(|| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.1.cmp(&b.1))
        });
        if rows.is_empty() {
            let row = if favorites_only {
                Self::status_row(
                    "ssh:no-favorites",
                    "status",
                    "No favorite SSH hosts",
                    Some("favorite a host from ssh list · ssh fav".into()),
                )
            } else if recent_only {
                Self::status_row(
                    "ssh:no-recent",
                    "status",
                    "No recent SSH connections",
                    Some("connect to a host to build history · ssh recent".into()),
                )
            } else if needle.is_empty() {
                Self::status_row(
                    "ssh:empty",
                    "status",
                    "No SSH hosts configured",
                    Some("Add Host entries to ~/.ssh/config".into()),
                )
            } else {
                Self::status_row(
                    "ssh:no-matches",
                    "status",
                    "No matching SSH hosts",
                    Some(format!("no matches for `{rest}`")),
                )
            };
            let mut out = vec![row];
            if let Some(reason) = meta_error.clone() {
                out.insert(0, Self::meta_unavailable_row(reason));
            }
            self.emit_results(&sink, out, vec![]).await;
            return;
        }

        let mut items = Vec::new();
        if let Some(reason) = meta_error {
            items.push(Self::meta_unavailable_row(reason));
        }
        if needle.is_empty() && !favorites_only && !recent_only {
            items.push(Self::status_row(
                "ssh:hint",
                "status",
                "ssh fav · ssh recent · ssh rename ALIAS NAME",
                Some("Enter connects in this terminal · reload with ssh reload".into()),
            ));
        }
        for (score, alias) in rows {
            if let Some(host) = self.resolve_host(&alias).await {
                let meta = meta_map.get(&alias);
                items.push(Self::host_item(&alias, &host, meta, score));
            }
        }
        self.emit_results(&sink, items, vec![]).await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind != "ssh_host" {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "—".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        let Some(alias) = Self::alias_from_item(result) else {
            return Self::host_actions(false);
        };
        let favorite = self.favorite_for_alias(&alias).await;
        Self::host_actions(favorite)
    }

    async fn preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind != "ssh_host" {
            return result.subtitle.clone();
        }
        let alias = Self::alias_from_item(result)?;
        let host = self.resolve_host(&alias).await?;
        let meta = self.meta_cache.read().await.get(&alias).cloned();
        let identity = host
            .identity_file
            .as_deref()
            .map(sanitize_identity_display)
            .unwrap_or_else(|| "-".into());
        let mut lines = vec![
            format!("alias: {alias}"),
            format!("hostname: {}", host.hostname.as_deref().unwrap_or("-")),
            format!("user: {}", host.user.as_deref().unwrap_or("-")),
            format!("port: {}", host.port.unwrap_or(22)),
            format!("identity file: {identity}"),
            format!("proxy jump: {}", host.proxy_jump.as_deref().unwrap_or("-")),
            format!(
                "connect timeout: {}",
                host.connect_timeout
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into())
            ),
        ];
        if let Some(m) = &meta {
            if let Some(name) = &m.display_name {
                lines.push(format!("display name: {name}"));
            }
            lines.push(format!(
                "favorite: {}",
                if m.favorite { "yes" } else { "no" }
            ));
            lines.push(format!("connection count: {}", m.connection_count));
            lines.push(format!(
                "last connected: {}",
                m.last_connected_at.as_deref().unwrap_or("-")
            ));
        } else {
            lines.push("favorite: no".into());
            lines.push("connection count: 0".into());
            lines.push("last connected: -".into());
        }
        Some(lines.join("\n"))
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let action_id = action.action.id.as_str();
        if action_id == "noop" {
            return ActionOutcome::Success { message: None };
        }
        if action_id == "reload_config" {
            return self.reload_config().await;
        }
        if action_id == "rename" {
            return self.apply_rename(&action.result).await;
        }
        if action_id == "record_connection" {
            let Some(alias) = action
                .result
                .action_payload
                .as_ref()
                .and_then(|p| p.get("alias"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
            else {
                return ActionOutcome::Failed {
                    kind: FailureKind::InvalidInput {
                        field: "alias".into(),
                        message: "missing alias".into(),
                    },
                };
            };
            let Some(meta) = &self.meta else {
                return ActionOutcome::Success { message: None };
            };
            if !self.alias_is_known(&alias).await {
                return ActionOutcome::Success { message: None };
            }
            let now = match self.clock.now_rfc3339() {
                Ok(ts) => ts,
                Err(err) => {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: false,
                        },
                    };
                }
            };
            return match meta.record_connection(&alias, &now) {
                Ok(()) => {
                    let _ = self.refresh().await;
                    ActionOutcome::Success {
                        message: Some("connection recorded".into()),
                    }
                }
                Err(err) => ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: false,
                    },
                },
            };
        }

        let Some(alias) = Self::alias_from_item(&action.result) else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: "invalid ssh host".into(),
                },
            };
        };

        if !self.alias_is_known(&alias).await {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: format!("unknown ssh host alias: {alias}"),
                },
            };
        }

        match action_id {
            "connect" => {
                if !self.config.ssh_available() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: "ssh command unavailable".into(),
                            retryable: false,
                        },
                    };
                }
                ActionOutcome::InteractiveTerminal {
                    program: "ssh".into(),
                    args: ssh_connect_args(&alias),
                    record_alias: Some(alias),
                }
            }
            "sftp" => {
                if !self.config.sftp_available() {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: "sftp command unavailable".into(),
                            retryable: false,
                        },
                    };
                }
                ActionOutcome::InteractiveTerminal {
                    program: "sftp".into(),
                    args: sftp_args(&alias),
                    record_alias: Some(alias),
                }
            }
            "favorite" => self.set_favorite(&alias, true).await,
            "unfavorite" => self.set_favorite(&alias, false).await,
            "copy_alias" => self.copy_alias(&alias, &cancel).await,
            "delete_metadata" => self.delete_metadata(&alias).await,
            _ => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{action_id}"),
                },
            },
        }
    }

    async fn teardown(&self) {
        *self.aliases.write().await = Vec::new();
        *self.resolved_cache.write().await = std::collections::HashMap::new();
        *self.meta_cache.write().await = std::collections::HashMap::new();
        *self.meta_error.write().await = None;
    }
}

impl SshModule {
    async fn set_favorite(&self, alias: &str, favorite: bool) -> ActionOutcome {
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: self
                        .meta_error
                        .read()
                        .await
                        .clone()
                        .unwrap_or_else(|| "ssh metadata store unavailable".into()),
                    retryable: false,
                },
            };
        };
        match meta.set_favorite(alias, favorite) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some(if favorite {
                        "favorited".into()
                    } else {
                        "unfavorited".into()
                    }),
                }
            }
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
        }
    }

    async fn copy_alias(&self, alias: &str, cancel: &CancellationToken) -> ActionOutcome {
        match await_unless_cancelled(cancel, self.pasteboard.write_text(alias)).await {
            Some(Ok(())) => ActionOutcome::Success {
                message: Some("copied alias".into()),
            },
            Some(Err(err)) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
            None => ActionOutcome::Cancelled,
        }
    }

    async fn reload_config(&self) -> ActionOutcome {
        self.resolved_cache.write().await.clear();
        let _ = self.refresh().await;
        ActionOutcome::Success {
            message: Some("SSH config cache reloaded".into()),
        }
    }

    async fn apply_rename(&self, result: &SearchItem) -> ActionOutcome {
        let Some(payload) = &result.action_payload else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "rename".into(),
                    message: "missing rename payload".into(),
                },
            };
        };
        let alias = payload
            .get("alias")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();
        let display_name = payload
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if alias.is_empty() {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: "missing alias".into(),
                },
            };
        }
        if !self.alias_is_known(alias).await {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: format!("unknown ssh host alias: {alias}"),
                },
            };
        }
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: "ssh metadata store unavailable".into(),
                    retryable: false,
                },
            };
        };
        match meta.set_display_name(alias, display_name) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some("display name saved".into()),
                }
            }
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
        }
    }

    async fn delete_metadata(&self, alias: &str) -> ActionOutcome {
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: "ssh metadata store unavailable".into(),
                    retryable: false,
                },
            };
        };
        match meta.delete(alias) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some("local metadata deleted".into()),
                }
            }
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::FixedClock;
    use luma_application::{
        FakePasteboard, FakeSshConfigPort, MemorySshMetaRepository, ResolvedSshHost, SshConfigState,
    };
    use tokio::sync::mpsc;

    fn sample_host(alias: &str) -> ResolvedSshHost {
        ResolvedSshHost {
            alias: alias.into(),
            hostname: Some("203.0.113.10".into()),
            user: Some("ubuntu".into()),
            port: Some(22),
            identity_file: Some("/home/u/.ssh/id_rsa".into()),
            proxy_jump: None,
            connect_timeout: None,
        }
    }

    fn test_module() -> SshModule {
        let config = Arc::new(FakeSshConfigPort::new().with_aliases(
            vec!["production", "staging"],
            vec![
                sample_host("production"),
                ResolvedSshHost {
                    alias: "staging".into(),
                    hostname: Some("staging.example.com".into()),
                    user: Some("deploy".into()),
                    port: Some(2222),
                    identity_file: None,
                    proxy_jump: Some("bastion".into()),
                    connect_timeout: Some(30),
                },
            ],
        ));
        SshModule::with_deps(
            config,
            Some(Arc::new(MemorySshMetaRepository::new())),
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock::new("2026-01-01", "2026-01-01T00:00:00Z")),
        )
    }

    async fn collect_search(module: &SshModule, query: &str) -> Vec<SearchItemDto> {
        let (tx, mut rx) = mpsc::channel(8);
        let q = luma_domain::Query::parse_with_prefixes(query, 50, |t| t == "ssh");
        module.search(q, tx, CancellationToken::new()).await;
        let mut items = Vec::new();
        while let Some(Event::ResultsChunk { upserts, .. }) = rx.recv().await {
            items.extend(upserts);
        }
        items
    }

    #[tokio::test]
    async fn search_lists_hosts() {
        let module = test_module();
        let items = collect_search(&module, "ssh").await;
        assert!(items.iter().any(|i| i.id == "ssh:production"));
    }

    #[tokio::test]
    async fn not_configured_when_missing_config() {
        let config = Arc::new(FakeSshConfigPort::new());
        config.set_state(SshConfigState::NotConfigured);
        let module = SshModule::with_deps(
            config,
            None,
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock::new("2026-01-01", "2026-01-01T00:00:00Z")),
        );
        let items = collect_search(&module, "ssh").await;
        assert!(items.iter().any(|i| i.kind == "not_configured"));
    }

    #[tokio::test]
    async fn connect_returns_interactive_terminal_args() {
        let module = test_module();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ssh:production"),
            module_id: ModuleId::new("luma.ssh"),
            title: "production".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "production" })),
        };
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("connect"),
                        label: "Connect".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        match outcome {
            ActionOutcome::InteractiveTerminal { program, args, .. } => {
                assert_eq!(program, "ssh");
                assert_eq!(args, vec!["production"]);
            }
            other => panic!("expected interactive, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn preview_hides_private_key_content() {
        let module = test_module();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ssh:production"),
            module_id: ModuleId::new("luma.ssh"),
            title: "production".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "production" })),
        };
        let body = module.preview(&item).await.unwrap();
        assert!(body.contains("id_rsa"));
        assert!(!body.contains("-----BEGIN"));
    }

    #[tokio::test]
    async fn favorite_and_record_metadata() {
        let module = test_module();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ssh:production"),
            module_id: ModuleId::new("luma.ssh"),
            title: "production".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("favorite"),
                label: "Favorite".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "production" })),
        };
        let out = module
            .perform(
                ActionRequest {
                    result: item.clone(),
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
        assert!(matches!(out, ActionOutcome::Success { .. }));

        let record = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        action_payload: Some(serde_json::json!({ "alias": "production" })),
                        ..item
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("record_connection"),
                        label: "Record".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        match record {
            ActionOutcome::Success { message } => {
                assert_eq!(message.as_deref(), Some("connection recorded"));
            }
            other => panic!("expected success, got {other:?}"),
        }

        let preview_item = SearchItem {
            id: luma_domain::ResultId::new("ssh:production"),
            module_id: ModuleId::new("luma.ssh"),
            title: "production".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "production" })),
        };
        let body = module.preview(&preview_item).await.unwrap();
        assert!(body.contains("favorite: yes"));
        assert!(body.contains("connection count: 1"));
        assert!(body.contains("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn parse_rename_accepts_case_insensitive_prefix_and_multi_word_name() {
        let p = parse_rename_query("Rename prod Production Server").unwrap();
        assert_eq!(p.alias, "prod");
        assert_eq!(p.display_name, "Production Server");
    }

    #[tokio::test]
    async fn connect_rejects_unknown_alias() {
        let module = test_module();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ssh:unknown"),
            module_id: ModuleId::new("luma.ssh"),
            title: "unknown".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("connect"),
                label: "Connect".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "unknown" })),
        };
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: ActionDescriptor {
                        id: ActionId::new("connect"),
                        label: "Connect".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::InvalidInput { .. },
            }
        ));
    }

    #[tokio::test]
    async fn meta_read_failure_clears_stale_cache() {
        let meta = Arc::new(MemorySshMetaRepository::new());
        meta.set_favorite("production", true).unwrap();
        let module = SshModule::with_deps(
            Arc::new(
                FakeSshConfigPort::new()
                    .with_aliases(vec!["production"], vec![sample_host("production")]),
            ),
            Some(meta.clone()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FixedClock::new("2026-01-01", "2026-01-01T00:00:00Z")),
        );
        let items = collect_search(&module, "ssh").await;
        let production = items
            .iter()
            .find(|i| i.id == "ssh:production")
            .expect("production row");
        assert!(production
            .subtitle
            .as_ref()
            .is_some_and(|s| s.contains('★')));

        meta.set_list_error(Some("disk error".into()));
        let items = collect_search(&module, "ssh").await;
        assert!(items.iter().any(|i| i.id == "ssh:meta-unavailable"));
        let production = items
            .iter()
            .find(|i| i.id == "ssh:production")
            .expect("production row");
        assert!(!production
            .subtitle
            .as_ref()
            .is_some_and(|s| s.contains('★')));
    }

    #[tokio::test]
    async fn favorites_filter_lists_only_starred() {
        let module = test_module();
        let item = SearchItem {
            id: luma_domain::ResultId::new("ssh:production"),
            module_id: ModuleId::new("luma.ssh"),
            title: "production".into(),
            subtitle: None,
            kind: "ssh_host".into(),
            score: 1.0,
            primary_action: ActionDescriptor {
                id: ActionId::new("favorite"),
                label: "Favorite".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: vec![],
            ui_intent: None,
            action_payload: Some(serde_json::json!({ "alias": "production" })),
        };
        let _ = module
            .perform(
                ActionRequest {
                    result: item,
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
        let items = collect_search(&module, "ssh fav").await;
        assert!(items.iter().any(|i| i.id == "ssh:production"));
        assert!(!items.iter().any(|i| i.id == "ssh:staging"));
    }
}
