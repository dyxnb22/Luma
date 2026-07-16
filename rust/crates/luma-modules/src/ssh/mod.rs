use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    sanitize_identity_display, sftp_args, ssh_connect_args, ActionOutcome, ActionRequest,
    ClockPort, LumaModule, ModuleManifest, ModuleState, PasteboardPort, ResolvedSshHost,
    SearchMode, SearchSink, SshConfigPort, SshHostMeta, SshMetaRepository, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
mod cache;
mod perform;
mod rename;
mod search;

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
        self.search_hosts(query, sink, cancel).await;
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

#[cfg(test)]
mod tests {
    use super::rename::parse_rename_query;
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
