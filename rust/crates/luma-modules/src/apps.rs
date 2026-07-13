use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{AppEntry, AppsCatalog};
use luma_protocol::{Event, SearchItemDto};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

struct AppsCache {
    apps: Vec<AppEntry>,
    warming: bool,
}

pub struct AppsModule {
    manifest: ModuleManifest,
    catalog: Arc<dyn AppsCatalog>,
    cache: Arc<RwLock<AppsCache>>,
}

impl AppsModule {
    pub fn new(catalog: Arc<dyn AppsCatalog>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.apps"),
                display_name: "Apps".into(),
                triggers: vec!["app".into(), "apps".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
            },
            catalog,
            cache: Arc::new(RwLock::new(AppsCache {
                apps: Vec::new(),
                warming: false,
            })),
        }
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
        match self.catalog.list_installed().await {
            Ok(apps) => {
                let mut g = self.cache.write().await;
                g.apps = apps;
                g.warming = false;
            }
            Err(_) => {
                let mut g = self.cache.write().await;
                g.warming = false;
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
        if g.apps.is_empty() {
            ModuleState::Cold
        } else {
            ModuleState::Ready
        }
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        // Memory-only path: if cold, emit warming diagnostic-like row and schedule refresh.
        let (apps, warming) = {
            let g = self.cache.read().await;
            (g.apps.clone(), g.warming || g.apps.is_empty())
        };

        if apps.is_empty() {
            if warming {
                let warm = SearchItemDto {
                    id: "apps:warming".into(),
                    module_id: "luma.apps".into(),
                    title: "App index warming…".into(),
                    subtitle: Some("cache refresh in progress".into()),
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
            }
            let catalog = self.catalog.clone();
            let cache = self.cache.clone();
            tokio::spawn(async move {
                if cancel.is_cancelled() {
                    return;
                }
                {
                    let mut g = cache.write().await;
                    g.warming = true;
                }
                if let Ok(apps) = catalog.list_installed().await {
                    let mut g = cache.write().await;
                    g.apps = apps;
                    g.warming = false;
                } else {
                    let mut g = cache.write().await;
                    g.warming = false;
                }
            });
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
            let mut upserts = Vec::new();
            for app in apps.into_iter().take(query.limit) {
                if cancel.is_cancelled() {
                    return;
                }
                upserts.push(SearchItemDto {
                    id: format!("app:{}", app.path.to_string_lossy()),
                    module_id: "luma.apps".into(),
                    title: app.name,
                    subtitle: Some(app.path.display().to_string()),
                    kind: "app".into(),
                    score: 60.0,
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
                        removed_ids: vec![],
                    })
                    .await;
            }
            return;
        }

        let mut upserts = Vec::new();
        for app in apps
            .into_iter()
            .filter(|a| a.name.to_lowercase().contains(&needle.to_lowercase()))
        {
            if cancel.is_cancelled() {
                return;
            }
            let id = format!("app:{}", app.path.to_string_lossy());
            upserts.push(SearchItemDto {
                id,
                module_id: "luma.apps".into(),
                title: app.name,
                subtitle: Some(app.path.display().to_string()),
                kind: "app".into(),
                score: 80.0,
                primary_action_id: "launch".into(),
                primary_action_label: "Launch".into(),
                ..Default::default()
            });
            if upserts.len() >= query.limit {
                break;
            }
        }

        if !upserts.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
        }
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
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
            "launch" => match await_unless_cancelled(&cancel, self.catalog.launch(&path)).await {
                None => ActionOutcome::Cancelled,
                Some(Ok(())) => ActionOutcome::Success {
                    message: Some(format!("launched {}", path.display())),
                },
                Some(Err(err)) => ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: true,
                    },
                },
            },
            "reveal" => match await_unless_cancelled(&cancel, self.catalog.reveal(&path)).await {
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
            },
            "copy_path" => {
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                ActionOutcome::Success {
                    message: Some(path.display().to_string()),
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
        g.apps.clear();
        g.warming = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use luma_platform_macos::{AppEntry, AppLaunchError, AppsCatalog};
    use std::path::{Path, PathBuf};
    use tokio::sync::mpsc;

    struct FakeCatalog {
        apps: Vec<AppEntry>,
    }

    #[async_trait]
    impl AppsCatalog for FakeCatalog {
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

    #[tokio::test]
    async fn search_uses_memory_cache() {
        let catalog = Arc::new(FakeCatalog {
            apps: vec![AppEntry {
                name: "Safari".into(),
                path: PathBuf::from("/Applications/Safari.app"),
                bundle_id: None,
            }],
        });
        let module = AppsModule::new(catalog);
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
        let module = AppsModule::new(catalog);
        module
            .warmup(WarmupContext {
                cancel: CancellationToken::new(),
            })
            .await;
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(Query::parse("app", 10), tx, CancellationToken::new())
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
}
