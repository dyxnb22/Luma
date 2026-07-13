use luma_domain::{ActionDescriptor, ActionRisk, ModuleId, ResultId, SearchItem};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchItemDto {
    pub id: String,
    pub module_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub kind: String,
    pub score: f64,
    pub primary_action_id: String,
    pub primary_action_label: String,
}

impl From<&SearchItem> for SearchItemDto {
    fn from(item: &SearchItem) -> Self {
        Self {
            id: item.id.as_str().to_string(),
            module_id: item.module_id.as_str().to_string(),
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            kind: item.kind.clone(),
            score: item.score,
            primary_action_id: item.primary_action.id.as_str().to_string(),
            primary_action_label: item.primary_action.label.clone(),
        }
    }
}

impl SearchItemDto {
    pub fn into_domain(self) -> SearchItem {
        SearchItem {
            id: ResultId::new(self.id),
            module_id: ModuleId::new(self.module_id),
            title: self.title,
            subtitle: self.subtitle,
            kind: self.kind,
            score: self.score,
            primary_action: ActionDescriptor {
                id: luma_domain::ActionId::new(self.primary_action_id),
                label: self.primary_action_label,
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            secondary_actions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcomeDto {
    Success { message: Option<String> },
    Failed { message: String },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionReady,
    SearchStarted {
        request_id: String,
    },
    ResultsReset {
        request_id: String,
    },
    ResultsChunk {
        request_id: String,
        sequence: u64,
        upserts: Vec<SearchItemDto>,
        removed_ids: Vec<String>,
    },
    SearchFinished {
        request_id: String,
        total: usize,
        elapsed_ms: u64,
    },
    SearchCancelled {
        request_id: String,
    },
    ActionsAvailable {
        result_id: String,
        actions: Vec<serde_json::Value>,
    },
    ActionStarted {
        operation_id: String,
    },
    ActionOutput {
        operation_id: String,
        stream: String,
        chunk: String,
    },
    ActionProgress {
        operation_id: String,
        current: u64,
        total: u64,
        message: String,
    },
    ActionFinished {
        operation_id: String,
        outcome: ActionOutcomeDto,
    },
    DiagnosticRaised {
        diagnostic: serde_json::Value,
    },
    SettingsChanged {
        version: u64,
        settings: serde_json::Value,
    },
    ModuleStateChanged {
        module_id: String,
        state: String,
    },
    Fatal {
        correlation_id: String,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finished_json_snake_case() {
        let event = Event::SearchFinished {
            request_id: "r1".into(),
            total: 2,
            elapsed_ms: 10,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"search_finished\""));
        assert!(json.contains("request_id"));
    }
}
