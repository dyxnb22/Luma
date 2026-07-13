use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_platform_macos::{MacOpenPath, OpenPath};
use luma_protocol::{Event, SearchItemDto};
use luma_storage::{QuicklinkRow, QuicklinksStore};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

type Link = QuicklinkRow;

pub struct QuicklinksModule {
    manifest: ModuleManifest,
    store: Arc<QuicklinksStore>,
    index: RwLock<Vec<Link>>,
    opener: Arc<dyn OpenPath>,
}

impl QuicklinksModule {
    pub fn new() -> Self {
        let store = QuicklinksStore::luma_next_default()
            .expect("QuicklinksModule::new requires writable LumaNext");
        Self::with_deps(Arc::new(store), Arc::new(MacOpenPath))
    }

    pub fn with_deps(store: Arc<QuicklinksStore>, opener: Arc<dyn OpenPath>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.quicklinks"),
                display_name: "Quicklinks".into(),
                triggers: vec!["ql".into(), "quicklinks".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
            },
            store,
            index: RwLock::new(Vec::new()),
            opener,
        }
    }

    async fn refresh_index(&self) -> Result<(), String> {
        *self.index.write().await = self.store.list().map_err(|err| err.to_string())?;
        Ok(())
    }

    async fn upsert(&self, trigger: &str, url: &str) -> Result<(), String> {
        self.store
            .upsert(trigger, url)
            .map_err(|err| err.to_string())?;
        self.refresh_index().await
    }

    async fn delete(&self, trigger: &str) -> Result<(), String> {
        self.store.delete(trigger).map_err(|err| err.to_string())?;
        self.refresh_index().await
    }
}

impl Default for QuicklinksModule {
    fn default() -> Self {
        Self::new()
    }
}

fn allowed_scheme(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")
}

#[async_trait]
impl LumaModule for QuicklinksModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        match self.refresh_index().await {
            Ok(()) => ModuleState::Ready,
            Err(err) => ModuleState::Failed(err),
        }
    }
    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let token = query.normalized.split_whitespace().next().unwrap_or("");
        let rest = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();

        if rest.starts_with("add ") {
            let parts: Vec<_> = rest.trim_start_matches("add ").split_whitespace().collect();
            if parts.len() >= 2 {
                let trigger = parts[0];
                let url = parts[1..].join(" ");
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: format!("ql:add:{trigger}"),
                            module_id: "luma.quicklinks".into(),
                            title: format!("Add {trigger}"),
                            subtitle: Some(url),
                            kind: "create".into(),
                            score: 95.0,
                            primary_action_id: "add".into(),
                            primary_action_label: "Add".into(),
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        }

        let links = self.index.read().await.clone();
        let mut upserts = Vec::new();
        for link in links {
            if cancel.is_cancelled() {
                return;
            }
            if link.trigger == token
                || (!rest.is_empty() && link.trigger.contains(&rest))
                || (token != "ql"
                    && token != "quicklinks"
                    && !token.is_empty()
                    && link.trigger.contains(token))
            {
                upserts.push(SearchItemDto {
                    id: format!("ql:{}", link.trigger),
                    module_id: "luma.quicklinks".into(),
                    title: link.trigger.clone(),
                    subtitle: Some(link.url.clone()),
                    kind: "quicklink".into(),
                    score: if link.trigger == token { 90.0 } else { 70.0 },
                    primary_action_id: "open".into(),
                    primary_action_label: "Open".into(),
                });
            }
        }
        if token == "ql" || token == "quicklinks" {
            upserts.push(SearchItemDto {
                id: "ql:manage".into(),
                module_id: "luma.quicklinks".into(),
                title: "Manage Quicklinks".into(),
                subtitle: Some("ql add <trigger> <url>".into()),
                kind: "open".into(),
                score: 1.0,
                primary_action_id: "open".into(),
                primary_action_label: "Open".into(),
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
    }
    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str().starts_with("ql:add:") {
            return vec![ActionDescriptor {
                id: ActionId::new("add"),
                label: "Add".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        let mut actions = vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }];
        if result.id.as_str().starts_with("ql:") && result.id.as_str() != "ql:manage" {
            actions.push(ActionDescriptor {
                id: ActionId::new("delete"),
                label: "Delete".into(),
                risk: ActionRisk::Destructive,
                confirmation: true,
            });
        }
        actions
    }
    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "add" => {
                let Some(trigger) = action.result.id.as_str().strip_prefix("ql:add:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected ql:add:<trigger>".into(),
                        },
                    };
                };
                let Some(url) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                if !allowed_scheme(&url) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "URL scheme not allowed".into(),
                        },
                    };
                }
                match self.upsert(trigger, &url).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("added {trigger}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "delete" => {
                if action.action.confirmation && !action.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "confirmation required".into(),
                        },
                    };
                }
                let Some(trigger) = action.result.id.as_str().strip_prefix("ql:") else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected ql:<trigger>".into(),
                        },
                    };
                };
                match self.delete(trigger).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("deleted {trigger}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Io { context: err },
                    },
                }
            }
            "open" => {
                if action.result.id.as_str() == "ql:manage" {
                    return ActionOutcome::Success {
                        message: Some("use ql add <trigger> <url>".into()),
                    };
                }
                let Some(url) = action.result.subtitle.clone() else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "url".into(),
                            message: "missing url".into(),
                        },
                    };
                };
                if !allowed_scheme(&url) {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: "URL scheme not allowed".into(),
                        },
                    };
                }
                match self.opener.open(Path::new(&url)).await {
                    Ok(()) => ActionOutcome::Success {
                        message: Some(format!("opened {url}")),
                    },
                    Err(err) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: err.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: format!("action:{other}"),
                },
            },
        }
    }
    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upsert_and_search_index() {
        let dir = tempdir().unwrap();
        let store = Arc::new(QuicklinksStore::with_path(dir.path().join("ql.sqlite")).unwrap());
        let m = QuicklinksModule::with_deps(
            store.clone(),
            Arc::new(luma_platform_macos::FakeOpenPath::new()),
        );
        m.upsert("docs", "https://example.com").await.unwrap();
        std::fs::remove_file(dir.path().join("ql.sqlite")).unwrap();
        assert!(m.index.read().await.iter().any(|l| l.trigger == "docs"));
    }
}
