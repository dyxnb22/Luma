//! Lightweight primary-action contract checks for modules under test.

use luma_application::LumaModule;
use luma_domain::SearchItem;
use luma_protocol::Event;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Assert every emitted search row's primary action matches `actions()` on
/// id, label, risk, and confirmation.
pub async fn assert_primary_actions_resolvable(module: &dyn LumaModule, query: luma_domain::Query) {
    let items = collect_search_items(module, query).await;
    for item in items {
        if item.kind == "warming"
            || item.kind == "unavailable"
            || item.primary_action.id.as_str() == "noop"
        {
            continue;
        }
        let actions = module.actions(&item).await;
        let Some(matched) = actions.iter().find(|a| a.id == item.primary_action.id) else {
            panic!(
                "module {} result {} primary {} missing from actions {:?}",
                module.manifest().id.as_str(),
                item.id.as_str(),
                item.primary_action.id.as_str(),
                actions.iter().map(|a| a.id.as_str()).collect::<Vec<_>>()
            );
        };
        assert_eq!(
            matched.label,
            item.primary_action.label,
            "module {} result {} primary label mismatch",
            module.manifest().id.as_str(),
            item.id.as_str()
        );
        assert_eq!(
            matched.risk,
            item.primary_action.risk,
            "module {} result {} primary risk mismatch",
            module.manifest().id.as_str(),
            item.id.as_str()
        );
        assert_eq!(
            matched.confirmation,
            item.primary_action.confirmation,
            "module {} result {} primary confirmation mismatch",
            module.manifest().id.as_str(),
            item.id.as_str()
        );
    }
}

/// Collect first-chunk SearchItems for a query (helper for contract suites).
pub async fn collect_search_items(
    module: &dyn LumaModule,
    query: luma_domain::Query,
) -> Vec<SearchItem> {
    let (tx, mut rx) = mpsc::channel(64);
    module.search(query, tx, CancellationToken::new()).await;
    let mut items = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let Event::ResultsChunk { upserts, .. } = ev {
            for dto in upserts {
                items.push(dto.into_domain());
            }
        }
    }
    items
}
