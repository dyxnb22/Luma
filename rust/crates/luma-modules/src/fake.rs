use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, SearchMode, SearchSink,
    WarmupContext,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, Query, SearchItem};
use luma_protocol::{Event, SearchItemDto};
use tokio_util::sync::CancellationToken;

pub struct FakeEchoModule {
    manifest: ModuleManifest,
}

impl FakeEchoModule {
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new("luma.fake"),
                display_name: "Fake Echo".into(),
                triggers: vec!["fake".into(), "echo".into()],
                default_enabled: false,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
            },
        }
    }
}

impl Default for FakeEchoModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LumaModule for FakeEchoModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() || query.normalized.is_empty() {
            return;
        }
        let payload = match &query.scope {
            luma_domain::QueryScope::Targeted { .. } => query
                .normalized
                .split_once(|c: char| c.is_whitespace())
                .map(|(_, rest)| rest.to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| query.normalized.clone()),
            luma_domain::QueryScope::Global => query.normalized.clone(),
        };
        let item = SearchItemDto {
            id: format!("fake:{}", payload),
            module_id: self.manifest.id.as_str().into(),
            title: format!("Echo: {payload}"),
            subtitle: Some("fake module (Phase 2)".into()),
            kind: "fake".into(),
            score: 50.0,
            primary_action_id: "open".into(),
            primary_action_label: "Open".into(),
        };
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts: vec![item],
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, _result: &SearchItem) -> Vec<ActionDescriptor> {
        vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }]
    }

    async fn perform(&self, action: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        ActionOutcome::Success {
            message: Some(format!(
                "performed {} on {}",
                action.action.id, action.result.id
            )),
        }
    }

    async fn teardown(&self) {}
}

/// Contract harness smoke: stable id across identical queries.
#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::Query;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn stable_result_id() {
        let module = FakeEchoModule::new();
        let (tx, mut rx) = mpsc::channel(4);
        module
            .search(Query::parse("hello", 10), tx, CancellationToken::new())
            .await;
        let ev = rx.recv().await.unwrap();
        match ev {
            Event::ResultsChunk { upserts, .. } => {
                assert_eq!(upserts[0].id, "fake:hello");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
