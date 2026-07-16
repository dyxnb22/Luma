use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, AppEntry, AppsCatalogPort, LumaModule, ModuleManifest,
    ModuleState, PasteboardPort, SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

struct AppsCache {
    apps: Vec<AppEntry>,
    warming: bool,
    /// Last catalog load error (cleared on success).
    catalog_error: Option<String>,
    /// path → launch count (session MRU; higher = more recent/frequent)
    launch_counts: std::collections::HashMap<String, u64>,
}

pub struct AppsModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn AppsCatalogPort>,
    pasteboard: Arc<dyn PasteboardPort>,
    cache: Arc<RwLock<AppsCache>>,
}

impl AppsModule {
    pub fn new(catalog: Arc<dyn AppsCatalogPort>, pasteboard: Arc<dyn PasteboardPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.apps"),
                display_name: "Apps".into(),
                triggers: vec!["app".into(), "apps".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("A".into()),
                    suggested_query: Some("app ".into()),
                    empty_hint: Some("app safari".into()),
                    supports_browse: false,
                },
            },
            catalog,
            pasteboard,
            cache: Arc::new(RwLock::new(AppsCache {
                apps: Vec::new(),
                warming: false,
                catalog_error: None,
                launch_counts: std::collections::HashMap::new(),
            })),
        }
    }

    fn fuzzy_score(name: &str, needle: &str, mru_boost: f64) -> Option<f64> {
        let name_l = name.to_lowercase();
        let needle_l = needle.to_lowercase();
        if needle_l.is_empty() {
            return Some(50.0 + mru_boost);
        }
        if name_l == needle_l {
            return Some(100.0 + mru_boost);
        }
        if name_l.starts_with(&needle_l) {
            return Some(92.0 + mru_boost);
        }
        if name_l.contains(&needle_l) {
            return Some(80.0 + mru_boost);
        }
        // subsequence match: "sf" matches "Safari"
        let mut it = name_l.chars();
        for ch in needle_l.chars() {
            loop {
                match it.next() {
                    Some(c) if c == ch => break,
                    Some(_) => continue,
                    None => return None,
                }
            }
        }
        Some(65.0 + mru_boost)
    }

    async fn ensure_refresh(&self, cancel: CancellationToken) {
        {
            let g = self.cache.read().await;
            if !g.apps.is_empty() || g.warming {
                return;
            }
        }
        {
            let mut g = self.cache.write().await;
            if !g.apps.is_empty() || g.warming {
                return;
            }
            g.warming = true;
        }
        if cancel.is_cancelled() {
            let mut g = self.cache.write().await;
            g.warming = false;
            return;
        }
        let listed = tokio::select! {
            _ = cancel.cancelled() => {
                let mut g = self.cache.write().await;
                g.warming = false;
                return;
            }
            result = self.catalog.list_installed() => result,
        };
        match listed {
            Ok(apps) => {
                let mut g = self.cache.write().await;
                g.apps = apps;
                g.warming = false;
                g.catalog_error = None;
            }
            Err(err) => {
                let mut g = self.cache.write().await;
                g.warming = false;
                g.catalog_error = Some(err);
            }
        }
    }
}

#[async_trait]
impl LumaModule for AppsModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, ctx: WarmupContext) -> ModuleState {
        self.ensure_refresh(ctx.cancel).await;
        let g = self.cache.read().await;
        if let Some(err) = &g.catalog_error {
            return ModuleState::Failed(err.clone());
        }
        if g.apps.is_empty() {
            ModuleState::Cold
        } else {
            ModuleState::Ready
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        let (apps, warming, empty, catalog_error) = {
            let g = self.cache.read().await;
            (
                g.apps.clone(),
                g.warming,
                g.apps.is_empty(),
                g.catalog_error.clone(),
            )
        };

        if let Some(err) = catalog_error {
            if empty && !warming {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "apps:unavailable".into(),
                            module_id: "luma.apps".into(),
                            title: "App catalog unavailable".into(),
                            subtitle: Some(crate::ux::friendly_store_error(&err)),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        let apps = if empty {
            // Emit warming row whether we own the refresh or another task does.
            let warm = SearchItemDto {
                id: "apps:warming".into(),
                module_id: "luma.apps".into(),
                title: "Loading apps…".into(),
                subtitle: Some("first scan can take a moment".into()),
                kind: "warming".into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "Wait".into(),
                ..Default::default()
            };
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![warm],
                    removed_ids: vec![],
                })
                .await;

            if warming {
                // Another search/warmup already owns refresh — do not scan again.
                return;
            }

            {
                let mut g = self.cache.write().await;
                if g.warming || !g.apps.is_empty() {
                    return;
                }
                g.warming = true;
            }
            let listed = tokio::select! {
                _ = cancel.cancelled() => {
                    let mut g = self.cache.write().await;
                    g.warming = false;
                    return;
                }
                result = self.catalog.list_installed() => result,
            };
            match listed {
                Ok(apps) => {
                    let mut g = self.cache.write().await;
                    g.apps = apps.clone();
                    g.warming = false;
                    g.catalog_error = None;
                    apps
                }
                Err(err) => {
                    let mut g = self.cache.write().await;
                    g.warming = false;
                    g.catalog_error = Some(err.clone());
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 2,
                            upserts: vec![SearchItemDto {
                                id: "apps:unavailable".into(),
                                module_id: "luma.apps".into(),
                                title: "App catalog unavailable".into(),
                                subtitle: Some(crate::ux::friendly_store_error(&err)),
                                kind: "unavailable".into(),
                                score: 0.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "Unavailable".into(),
                                ..Default::default()
                            }],
                            removed_ids: vec!["apps:warming".into()],
                        })
                        .await;
                    return;
                }
            }
        } else {
            apps
        };

        if cancel.is_cancelled() {
            return;
        }

        let needle = match &query.scope {
            luma_domain::QueryScope::Targeted { .. } => query
                .normalized
                .split_once(|c: char| c.is_whitespace())
                .map(|(_, rest)| rest.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_default(),
            luma_domain::QueryScope::Global => query.normalized.clone(),
        };

        if needle.is_empty() {
            let counts = {
                let g = self.cache.read().await;
                g.launch_counts.clone()
            };
            let mut ranked = apps;
            ranked.sort_by(|a, b| {
                let ca = counts
                    .get(&a.path.to_string_lossy().to_string())
                    .copied()
                    .unwrap_or(0);
                let cb = counts
                    .get(&b.path.to_string_lossy().to_string())
                    .copied()
                    .unwrap_or(0);
                cb.cmp(&ca)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            let mut upserts = Vec::new();
            for app in ranked.into_iter().take(query.limit) {
                if cancel.is_cancelled() {
                    return;
                }
                let key = app.path.to_string_lossy().to_string();
                let mru = counts.get(&key).copied().unwrap_or(0) as f64;
                upserts.push(SearchItemDto {
                    id: format!("app:{}", app.path.to_string_lossy()),
                    module_id: "luma.apps".into(),
                    title: app.name,
                    subtitle: Some(app.path.display().to_string()),
                    kind: "app".into(),
                    score: 60.0 + mru.min(20.0),
                    primary_action_id: "launch".into(),
                    primary_action_label: "Launch".into(),
                    ..Default::default()
                });
            }
            if !upserts.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts,
                        removed_ids: vec!["apps:warming".into()],
                    })
                    .await;
            } else {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "app:empty".into(),
                            module_id: "luma.apps".into(),
                            title: "No apps indexed".into(),
                            subtitle: Some("Catalog refresh finished with an empty list".into()),
                            kind: "status".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec!["apps:warming".into()],
                    })
                    .await;
            }
            return;
        }

        let counts = {
            let g = self.cache.read().await;
            g.launch_counts.clone()
        };
        let mut scored: Vec<(f64, AppEntry)> = apps
            .into_iter()
            .filter_map(|app| {
                let key = app.path.to_string_lossy().to_string();
                let mru = counts.get(&key).copied().unwrap_or(0) as f64 * 0.5;
                Self::fuzzy_score(&app.name, &needle, mru.min(10.0)).map(|s| (s, app))
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut upserts = Vec::new();
        for (score, app) in scored.into_iter().take(query.limit) {
            if cancel.is_cancelled() {
                return;
            }
            upserts.push(SearchItemDto {
                id: format!("app:{}", app.path.to_string_lossy()),
                module_id: "luma.apps".into(),
                title: app.name,
                subtitle: Some(app.path.display().to_string()),
                kind: "app".into(),
                score,
                primary_action_id: "launch".into(),
                primary_action_label: "Launch".into(),
                ..Default::default()
            });
        }

        if !upserts.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec!["apps:warming".into()],
                })
                .await;
        } else if !needle.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "app:no-matches".into(),
                        module_id: "luma.apps".into(),
                        title: format!("No apps matching \"{needle}\""),
                        subtitle: Some("Try another name · app ".into()),
                        kind: "status".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "OK".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec!["apps:warming".into()],
                })
                .await;
        } else {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![],
                    removed_ids: vec!["apps:warming".into()],
                })
                .await;
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "app:no-matches"
            || result.id.as_str() == "app:empty"
            || result.kind == "status"
            || result.kind == "unavailable"
            || result.kind == "warming"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        vec![
            ActionDescriptor {
                id: ActionId::new("launch"),
                label: "Launch".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("reveal"),
                label: "Reveal in Finder".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptor {
                id: ActionId::new("copy_path"),
                label: "Copy Path".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
        ]
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        let path = action
            .result
            .id
            .as_str()
            .strip_prefix("app:")
            .map(std::path::PathBuf::from);
        let Some(path) = path else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "result_id".into(),
                    message: "expected app:<path>".into(),
                },
            };
        };

        match action.action.id.as_str() {
            "launch" => {
                let in_cache = {
                    let g = self.cache.read().await;
                    g.apps.iter().any(|a| a.path == path)
                };
                if !in_cache {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "app not in catalog cache".into(),
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.catalog.launch(&path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => {
                        let key = path.to_string_lossy().to_string();
                        let mut g = self.cache.write().await;
                        *g.launch_counts.entry(key).or_insert(0) += 1;
                        ActionOutcome::Success {
                            message: Some(format!("launched {}", path.display())),
                        }
                    }
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "reveal" => {
                let in_cache = {
                    let g = self.cache.read().await;
                    g.apps.iter().any(|a| a.path == path)
                };
                if !in_cache {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "app not in catalog cache".into(),
                        },
                    };
                }
                match await_unless_cancelled(&cancel, self.catalog.reveal(&path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("revealed".into()),
                    },
                    Some(Err(err)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "copy_path" => {
                let in_cache = {
                    let g = self.cache.read().await;
                    g.apps.iter().any(|a| a.path == path)
                };
                if !in_cache {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "app not in catalog cache".into(),
                        },
                    };
                }
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                let text = path.display().to_string();
                match self.pasteboard.write_text(&text).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("copied {text}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            "noop" => ActionOutcome::Success { message: None },
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }

    async fn teardown(&self) {
        let mut g = self.cache.write().await;
        g.apps = Vec::new();
        g.launch_counts = std::collections::HashMap::new();
        g.warming = false;
        g.catalog_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_application::{AppLaunchError, AppsCatalogPort, ModuleState, PasteboardError};
    use std::path::{Path, PathBuf};
    use tokio::sync::mpsc;
    use tokio::sync::Mutex as TokioMutex;

    struct FakeCatalog {
        apps: Vec<AppEntry>,
    }

    #[async_trait]
    impl AppsCatalogPort for FakeCatalog {
        async fn list_installed(&self) -> Result<Vec<AppEntry>, String> {
            Ok(self.apps.clone())
        }
        async fn launch(&self, _path: &Path) -> Result<(), AppLaunchError> {
            Ok(())
        }
        async fn reveal(&self, _path: &Path) -> Result<(), AppLaunchError> {
            Ok(())
        }
    }

    struct MemPb(TokioMutex<Option<String>>);

    #[async_trait]
    impl PasteboardPort for MemPb {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(self.0.lock().await.clone())
        }
        async fn write_text(&self, text: &str) -> Result<(), PasteboardError> {
            *self.0.lock().await = Some(text.into());
            Ok(())
        }
    }

    fn mem_pb() -> Arc<MemPb> {
        Arc::new(MemPb(TokioMutex::new(None)))
    }

    #[tokio::test]
    async fn search_uses_memory_cache() {
        let catalog = Arc::new(FakeCatalog {
            apps: vec![AppEntry {
                name: "Safari".into(),
                path: PathBuf::from("/Applications/Safari.app"),
                bundle_id: None,
            }],
        });
        let module = AppsModule::new(catalog, mem_pb());
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(Query::parse("app safari", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts.len(), 1);
                assert_eq!(upserts[0].title, "Safari");
                assert!(upserts[0].id.starts_with("app:"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn exact_trigger_lists_cached_apps() {
        let catalog = Arc::new(FakeCatalog {
            apps: vec![AppEntry {
                name: "Safari".into(),
                path: PathBuf::from("/Applications/Safari.app"),
                bundle_id: None,
            }],
        });
        let module = AppsModule::new(catalog, mem_pb());
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(Query::parse("app ", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts.len(), 1);
                assert_eq!(upserts[0].title, "Safari");
            }
            other => panic!("expected apps for exact trigger, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn copy_path_writes_pasteboard() {
        let catalog = Arc::new(FakeCatalog {
            apps: vec![AppEntry {
                name: "Safari".into(),
                path: PathBuf::from("/Applications/Safari.app"),
                bundle_id: None,
            }],
        });
        let pb = mem_pb();
        let module = AppsModule::new(catalog, pb.clone());
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let outcome = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("app:/Applications/Safari.app"),
                        module_id: ModuleId::new("luma.apps"),
                        title: "Safari".into(),
                        subtitle: None,
                        kind: "app".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("copy_path"),
                            label: "Copy Path".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("copy_path"),
                        label: "Copy Path".into(),
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
            pb.read_text().await.unwrap().as_deref(),
            Some("/Applications/Safari.app")
        );
    }

    struct FailingCatalog;

    #[async_trait]
    impl AppsCatalogPort for FailingCatalog {
        async fn list_installed(&self) -> Result<Vec<AppEntry>, String> {
            Err("catalog boom".into())
        }
        async fn launch(&self, _path: &Path) -> Result<(), AppLaunchError> {
            Ok(())
        }
        async fn reveal(&self, _path: &Path) -> Result<(), AppLaunchError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn catalog_failure_emits_unavailable_row() {
        let module = AppsModule::new(Arc::new(FailingCatalog), mem_pb());
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(Query::parse("app ", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts.len(), 1);
                assert_eq!(upserts[0].kind, "unavailable");
                assert!(upserts[0].subtitle.as_deref().unwrap().contains("boom"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn warmup_returns_failed_on_catalog_error() {
        let module = AppsModule::new(Arc::new(FailingCatalog), mem_pb());
        let state = module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        assert!(matches!(state, ModuleState::Failed(_)));
    }

    #[tokio::test]
    async fn launch_rejects_path_not_in_cache() {
        let catalog = Arc::new(FakeCatalog {
            apps: vec![AppEntry {
                name: "Safari".into(),
                path: PathBuf::from("/Applications/Safari.app"),
                bundle_id: None,
            }],
        });
        let module = AppsModule::new(catalog, mem_pb());
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let outcome = module
            .perform(
                ActionRequest {
                    result: SearchItem {
                        id: luma_domain::ResultId::new("app:/Applications/Other.app"),
                        module_id: ModuleId::new("luma.apps"),
                        title: "Other".into(),
                        subtitle: None,
                        kind: "app".into(),
                        score: 1.0,
                        primary_action: ActionDescriptor {
                            id: ActionId::new("launch"),
                            label: "Launch".into(),
                            risk: ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    },
                    action: ActionDescriptor {
                        id: ActionId::new("launch"),
                        label: "Launch".into(),
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
                kind: FailureKind::SecurityDenied { .. },
                ..
            }
        ));
    }
}
