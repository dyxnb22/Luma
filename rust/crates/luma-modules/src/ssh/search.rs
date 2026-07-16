use super::rename::parse_rename_query;
use super::SshModule;
use luma_application::{
    format_connection_subtitle, ResolvedSshHost, SearchSink, SshConfigState, SshHostMeta,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, Query, SearchItem};
use luma_protocol::{Event, SearchItemDto};
use tokio_util::sync::CancellationToken;

impl SshModule {
    pub(super) fn alias_from_item(item: &SearchItem) -> Option<String> {
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

    pub(super) fn host_row_id(alias: &str) -> String {
        format!("ssh:{alias}")
    }

    pub(super) fn fuzzy_match(needle: &str, hay: &str) -> bool {
        hay.to_lowercase().contains(&needle.to_lowercase())
    }

    pub(super) fn score_host(
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

    pub(super) fn compare_last_connected(
        a: Option<&SshHostMeta>,
        b: Option<&SshHostMeta>,
    ) -> std::cmp::Ordering {
        let ts_a = a.and_then(|m| m.last_connected_at.as_deref()).unwrap_or("");
        let ts_b = b.and_then(|m| m.last_connected_at.as_deref()).unwrap_or("");
        ts_b.cmp(ts_a)
    }

    pub(super) fn host_actions(favorite: bool) -> Vec<ActionDescriptor> {
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

    pub(super) fn meta_unavailable_row(reason: String) -> SearchItem {
        Self::status_row(
            "ssh:meta-unavailable",
            "unavailable",
            "SSH metadata unavailable",
            Some(reason),
        )
    }

    pub(super) async fn emit_results(
        &self,
        sink: &SearchSink,
        items: Vec<SearchItem>,
        removed: Vec<String>,
    ) {
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

    pub(super) fn status_row(
        id: &str,
        kind: &str,
        title: &str,
        subtitle: Option<String>,
    ) -> SearchItem {
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

    pub(super) fn host_item(
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

    pub(super) async fn search_hosts(
        &self,
        query: Query,
        sink: SearchSink,
        _cancel: CancellationToken,
    ) {
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
}
