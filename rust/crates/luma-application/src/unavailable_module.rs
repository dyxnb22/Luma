use crate::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchSink,
    WarmupContext,
};
use async_trait::async_trait;
use luma_domain::{ActionDescriptor, FailureKind, Query, QueryScope, SearchItem};
use luma_protocol::{Event, SearchItemDto};
use tokio_util::sync::CancellationToken;

/// Keeps a module's trigger and Hub entry available when its local store could
/// not be opened. This avoids silently routing `/clip` (for example) into a
/// global search after a corrupt or read-only database is detected.
pub struct UnavailableModule {
    manifest: ModuleManifest,
    reason: String,
}

impl UnavailableModule {
    pub fn new(manifest: ModuleManifest, reason: impl Into<String>) -> Self {
        Self {
            manifest,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl LumaModule for UnavailableModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Failed(self.reason.clone())
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() || !matches!(query.scope, QueryScope::Targeted { .. }) {
            return;
        }
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![SearchItemDto {
                    id: format!("{}:unavailable", self.manifest.id.as_str()),
                    module_id: self.manifest.id.as_str().to_string(),
                    title: format!("{} is unavailable", self.manifest.display_name),
                    subtitle: Some(self.reason.clone()),
                    kind: "unavailable".into(),
                    score: 0.0,
                    // The TUI deliberately renders unavailable items as
                    // informational; this field preserves the protocol shape
                    // for CLI/API consumers without advertising a fake action.
                    primary_action_id: "status".into(),
                    primary_action_label: "Status".into(),
                    ..Default::default()
                }],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        Vec::new()
    }

    async fn perform(&self, _action: ActionRequest, _cancel: CancellationToken) -> ActionOutcome {
        ActionOutcome::Failed {
            kind: FailureKind::Unavailable {
                reason: self.reason.clone(),
                retryable: true,
            },
        }
    }

    async fn teardown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SearchMode, WorkbenchMeta};
    use luma_domain::ModuleId;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn targeted_search_reports_unavailable_state() {
        let module = UnavailableModule::new(
            ModuleManifest {
                id: ModuleId::new("luma.clipboard"),
                display_name: "Clipboard".into(),
                triggers: vec!["clip".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: WorkbenchMeta::default(),
            },
            "Clipboard store could not be opened; data was left untouched.",
        );
        let (tx, mut rx) = mpsc::channel(1);

        module
            .search(Query::parse("clip ", 10), tx, CancellationToken::new())
            .await;

        let Event::ResultsChunk { upserts, .. } = rx.recv().await.expect("status row") else {
            panic!("expected a result chunk");
        };
        assert_eq!(upserts[0].kind, "unavailable");
        assert!(upserts[0]
            .subtitle
            .as_deref()
            .is_some_and(|text| text.contains("left untouched")));
    }
}
