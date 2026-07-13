//! Shared test helpers. No production I/O.

use luma_domain::{ActionDescriptor, ActionId, ActionRisk, ModuleId, ResultId, SearchItem};
use luma_protocol::SearchItemDto;

pub fn sample_search_item(id: &str, title: &str, score: f64) -> SearchItem {
    SearchItem {
        id: ResultId::new(id),
        module_id: ModuleId::new("mock"),
        title: title.into(),
        subtitle: None,
        kind: "mock".into(),
        score,
        primary_action: ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        },
        secondary_actions: Vec::new(),
    }
}

pub fn sample_dto(id: &str, title: &str, score: f64) -> SearchItemDto {
    SearchItemDto::from(&sample_search_item(id, title, score))
}
