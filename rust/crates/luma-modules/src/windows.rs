//! Windows module — list + focus visible windows; Hub projects previous-frontmost.

use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, HubWindowRow, HubWindowsSlice, HubWindowsStatus, LumaModule,
    ModuleManifest, ModuleState, SearchMode, SearchSink, WarmupContext, WindowCatalogPort,
    WindowEntry, WindowError,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Hard cap for Hub window rows (ADR-0004).
pub const HUB_WINDOWS_MAX: usize = 15;

pub struct WindowsModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn WindowCatalogPort>,
    cache: Arc<RwLock<Vec<WindowEntry>>>,
}

impl WindowsModule {
    pub fn with_catalog(catalog: Arc<dyn WindowCatalogPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.windows"),
                display_name: "Windows".into(),
                triggers: vec!["win".into(), "window".into(), "windows".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec!["accessibility".into()],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("W".into()),
                    suggested_query: Some("win ".into()),
                    empty_hint: Some("win · focus a window".into()),
                    supports_browse: false,
                },
            },
            catalog,
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn result_id(entry: &WindowEntry) -> String {
        format!("win:{}", entry.id)
    }

    fn parse_window_id(result_id: &str) -> Option<&str> {
        result_id.strip_prefix("win:")
    }

    fn entry_to_dto(entry: &WindowEntry, score: f64) -> SearchItemDto {
        SearchItemDto {
            id: Self::result_id(entry),
            module_id: "luma.windows".into(),
            title: entry.title.clone(),
            subtitle: Some(entry.app_name.clone()),
            kind: "window".into(),
            score,
            primary_action_id: "focus".into(),
            primary_action_label: "Focus".into(),
            primary_action_risk: ActionRisk::Safe,
            primary_action_confirmation: false,
            ..Default::default()
        }
    }

    fn matches(entry: &WindowEntry, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        let hay = format!("{} {}", entry.title, entry.app_name).to_lowercase();
        hay.contains(needle)
    }

    fn map_list_error(err: &WindowError) -> SearchItemDto {
        match err {
            WindowError::PermissionRequired {
                capability,
                guidance,
            } => SearchItemDto {
                id: "win:permission".into(),
                module_id: "luma.windows".into(),
                title: format!("Permission required ({capability})"),
                subtitle: Some(guidance.clone()),
                kind: "permission_required".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            },
            WindowError::Unavailable(reason) | WindowError::NotFound(reason) => SearchItemDto {
                id: "win:unavailable".into(),
                module_id: "luma.windows".into(),
                title: "Window list unavailable".into(),
                subtitle: Some(reason.clone()),
                kind: "unavailable".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "Unavailable".into(),
                ..Default::default()
            },
        }
    }

    fn map_focus_error(err: WindowError) -> ActionOutcome {
        match err {
            WindowError::PermissionRequired {
                capability,
                guidance,
            } => ActionOutcome::Failed {
                kind: FailureKind::PermissionRequired {
                    capability,
                    guidance,
                },
            },
            WindowError::NotFound(entity) => ActionOutcome::Failed {
                kind: FailureKind::NotFound { entity },
            },
            WindowError::Unavailable(reason) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason,
                    retryable: true,
                },
            },
        }
    }

    async fn refresh_cache(&self) -> Result<Vec<WindowEntry>, WindowError> {
        let list = self.catalog.list_windows().await?;
        *self.cache.write().await = list.clone();
        Ok(list)
    }
}

#[async_trait]
impl LumaModule for WindowsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        if ctx.cancel.is_cancelled() {
            return ModuleState::Cold;
        }
        match self.refresh_cache().await {
            Ok(_) => ModuleState::Ready,
            Err(err) => ModuleState::Failed(err.to_string()),
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let needle = query.rest_normalized();
        let mut list = self.cache.read().await.clone();
        if list.is_empty() {
            let warming = SearchItemDto {
                id: "win:warming".into(),
                module_id: "luma.windows".into(),
                title: "Refreshing windows…".into(),
                subtitle: Some("warming".into()),
                kind: "warming".into(),
                score: 0.0,
                primary_action_id: "refresh".into(),
                primary_action_label: "Refresh".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![warming],
                    removed_ids: vec![],
                })
                .await;
            let listed = tokio::select! {
                _ = cancel.cancelled() => return,
                result = self.refresh_cache() => result,
            };
            match listed {
                Ok(fresh) => list = fresh,
                Err(err) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 2,
                            upserts: vec![Self::map_list_error(&err)],
                            removed_ids: vec!["win:warming".into()],
                        })
                        .await;
                    return;
                }
            }
        }

        // Group-ish: sort by app then title for stable browsing.
        list.sort_by(|a, b| {
            a.app_name
                .to_lowercase()
                .cmp(&b.app_name.to_lowercase())
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });

        let mut upserts = Vec::new();
        for entry in list {
            if cancel.is_cancelled() {
                return;
            }
            if Self::matches(&entry, &needle) {
                upserts.push(Self::entry_to_dto(&entry, 50.0));
            }
            if upserts.len() >= query.limit {
                break;
            }
        }

        if upserts.is_empty() {
            let title = if needle.is_empty() {
                "No windows to list".into()
            } else {
                format!("No windows matching \"{needle}\"")
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "win:no-matches".into(),
                        module_id: "luma.windows".into(),
                        title,
                        subtitle: Some(
                            "Try a different filter or grant Screen Recording for titles".into(),
                        ),
                        kind: "status".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec!["win:warming".into()],
                })
                .await;
        } else {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec!["win:warming".into()],
                })
                .await;
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        match result.kind.as_str() {
            "warming" => vec![ActionDescriptor {
                id: ActionId::new("refresh"),
                label: "Refresh".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            "status" | "unavailable" | "permission_required" => vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            _ if result.primary_action.id.as_str() == "noop" => vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
            _ => vec![ActionDescriptor {
                id: ActionId::new("focus"),
                label: "Focus".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }],
        }
    }

    async fn hub_windows(&self) -> Option<HubWindowsSlice> {
        let app_name = self
            .catalog
            .previous_frontmost_app()
            .await
            .unwrap_or_else(|| "Windows".into());
        let list = match self.catalog.list_windows().await {
            Ok(list) => list,
            Err(err) => {
                let status = match err {
                    WindowError::PermissionRequired {
                        capability,
                        guidance,
                    } => HubWindowsStatus {
                        kind: "permission_required".into(),
                        title: format!("Permission required ({capability})"),
                        subtitle: Some(guidance),
                    },
                    WindowError::Unavailable(reason) | WindowError::NotFound(reason) => {
                        HubWindowsStatus {
                            kind: "unavailable".into(),
                            title: "Window list unavailable".into(),
                            subtitle: Some(reason),
                        }
                    }
                };
                return Some(HubWindowsSlice {
                    app_name,
                    windows: Vec::new(),
                    more: None,
                    status: Some(status),
                });
            }
        };
        let previous = self.catalog.previous_frontmost_app().await?;
        let mut for_app: Vec<_> = list
            .into_iter()
            .filter(|e| e.app_name.eq_ignore_ascii_case(&previous))
            .collect();
        for_app.sort_by_key(|a| a.title.to_lowercase());
        let total = for_app.len();
        let more = if total > HUB_WINDOWS_MAX {
            Some((total - HUB_WINDOWS_MAX) as u32)
        } else {
            None
        };
        let windows = for_app
            .into_iter()
            .take(HUB_WINDOWS_MAX)
            .map(|e| HubWindowRow {
                id: Self::result_id(&e),
                title: e.title,
            })
            .collect();
        Some(HubWindowsSlice {
            app_name: previous,
            windows,
            more,
            status: None,
        })
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "refresh" => match self.refresh_cache().await {
                Ok(_) => ActionOutcome::Success {
                    message: Some("refreshed".into()),
                },
                Err(err) => Self::map_focus_error(err),
            },
            "focus" => {
                let Some(window_id) = Self::parse_window_id(action.result.id.as_str()) else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected win:<window_id>".into(),
                        },
                    };
                };
                // Refresh once before focus (ADR-0004: no background poll).
                let _ = self.refresh_cache().await;
                match await_unless_cancelled(&cancel, self.catalog.focus(window_id)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some(format!("focused {}", action.result.title)),
                    },
                    Some(Err(err)) => Self::map_focus_error(err),
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
        self.cache.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::FakeWindowCatalog;
    use luma_domain::{ActionId, Query};
    use tokio::sync::mpsc;

    fn sample(id: &str, app: &str, title: &str) -> WindowEntry {
        WindowEntry {
            id: id.into(),
            app_name: app.into(),
            app_bundle_id: None,
            title: title.into(),
            is_on_screen: true,
            layer: 0,
            owner_pid: 1,
        }
    }

    #[tokio::test]
    async fn search_filters_by_title_and_app() {
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
            vec![
                sample("pid:1|num:1", "Cursor", "Luma"),
                sample("pid:1|num:2", "Cursor", "other-project"),
                sample("pid:2|num:1", "Safari", "Apple"),
            ],
            Some("Cursor".into()),
        ));
        let m = WindowsModule::with_catalog(catalog);
        m.warmup(WarmupContext {
            cancel: CancellationToken::new(),
        })
        .await;
        let (tx, mut rx) = mpsc::channel(8);
        m.search(Query::parse("win luma", 20), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected chunk");
        };
        assert_eq!(upserts.len(), 1);
        assert_eq!(upserts[0].title, "Luma");
        assert_eq!(upserts[0].primary_action_id, "focus");
    }

    #[tokio::test]
    async fn permission_error_surfaces_row() {
        let catalog = Arc::new(FakeWindowCatalog::default());
        *catalog.list_error.lock().await = Some(WindowError::PermissionRequired {
            capability: "screen_recording".into(),
            guidance: "Grant Screen Recording".into(),
        });
        let m = WindowsModule::with_catalog(catalog);
        let (tx, mut rx) = mpsc::channel(8);
        m.search(Query::parse("win ", 20), tx, CancellationToken::new())
            .await;
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected warming");
        };
        assert_eq!(upserts[0].kind, "warming");
        let Event::ResultsChunk { upserts, .. } = rx.recv().await.unwrap() else {
            panic!("expected permission");
        };
        assert_eq!(upserts[0].kind, "permission_required");
    }

    #[tokio::test]
    async fn focus_records_call_on_fake() {
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
            vec![sample("pid:1|num:1", "Cursor", "Luma")],
            Some("Cursor".into()),
        ));
        let m = WindowsModule::with_catalog(catalog.clone());
        let outcome = m
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("win:pid:1|num:1"),
                        module_id: ModuleId::new("luma.windows"),
                        title: "Luma".into(),
                        subtitle: Some("Cursor".into()),
                        kind: "window".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("focus"),
                            label: "Focus".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("focus"),
                        label: "Focus".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Success { .. }));
        assert_eq!(
            catalog.focus_calls.lock().await.as_slice(),
            &["pid:1|num:1".to_string()]
        );
    }

    #[tokio::test]
    async fn hub_windows_caps_and_filters_app() {
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push(sample(
                &format!("pid:1|num:{i}"),
                "Cursor",
                &format!("w{i}"),
            ));
        }
        entries.push(sample("pid:2|num:1", "Safari", "other"));
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
            entries,
            Some("Cursor".into()),
        ));
        let m = WindowsModule::with_catalog(catalog);
        let slice = m.hub_windows().await.unwrap();
        assert_eq!(slice.app_name, "Cursor");
        assert_eq!(slice.windows.len(), HUB_WINDOWS_MAX);
        assert_eq!(slice.more, Some(5));
        assert!(slice.status.is_none());
    }

    #[tokio::test]
    async fn hub_windows_surfaces_list_permission() {
        let catalog = Arc::new(FakeWindowCatalog::with_entries(
            vec![],
            Some("Cursor".into()),
        ));
        *catalog.list_error.lock().await = Some(WindowError::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Grant AX".into(),
        });
        let m = WindowsModule::with_catalog(catalog);
        let slice = m.hub_windows().await.unwrap();
        assert!(slice.windows.is_empty());
        let status = slice.status.unwrap();
        assert_eq!(status.kind, "permission_required");
    }
}
