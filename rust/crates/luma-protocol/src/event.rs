use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, ResultId, SearchItem,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDescriptorDto {
    pub id: String,
    pub label: String,
    pub risk: ActionRisk,
    pub confirmation: bool,
}

impl From<&ActionDescriptor> for ActionDescriptorDto {
    fn from(action: &ActionDescriptor) -> Self {
        Self {
            id: action.id.as_str().to_string(),
            label: action.label.clone(),
            risk: action.risk.clone(),
            confirmation: action.confirmation,
        }
    }
}

impl ActionDescriptorDto {
    pub fn into_domain(self) -> ActionDescriptor {
        ActionDescriptor {
            id: ActionId::new(self.id),
            label: self.label,
            risk: self.risk,
            confirmation: self.confirmation,
        }
    }

    pub fn needs_confirmation(&self) -> bool {
        self.confirmation || !matches!(self.risk, ActionRisk::Safe)
    }
}

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
    #[serde(default)]
    pub primary_action_risk: ActionRisk,
    #[serde(default)]
    pub primary_action_confirmation: bool,
    #[serde(default)]
    pub secondary_actions: Vec<ActionDescriptorDto>,
}

impl Default for SearchItemDto {
    fn default() -> Self {
        Self {
            id: String::new(),
            module_id: String::new(),
            title: String::new(),
            subtitle: None,
            kind: String::new(),
            score: 0.0,
            primary_action_id: String::new(),
            primary_action_label: String::new(),
            primary_action_risk: ActionRisk::Safe,
            primary_action_confirmation: false,
            secondary_actions: Vec::new(),
        }
    }
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
            primary_action_risk: item.primary_action.risk.clone(),
            primary_action_confirmation: item.primary_action.confirmation,
            secondary_actions: item
                .secondary_actions
                .iter()
                .map(ActionDescriptorDto::from)
                .collect(),
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
                id: ActionId::new(self.primary_action_id),
                label: self.primary_action_label,
                risk: self.primary_action_risk,
                confirmation: self.primary_action_confirmation,
            },
            secondary_actions: self
                .secondary_actions
                .into_iter()
                .map(ActionDescriptorDto::into_domain)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcomeDto {
    Success {
        message: Option<String>,
    },
    Failed {
        #[serde(flatten)]
        kind: FailureKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Cancelled,
}

impl ActionOutcomeDto {
    pub fn failed(kind: FailureKind) -> Self {
        let message = Some(kind.display_message());
        Self::Failed { kind, message }
    }

    pub fn display_message(&self) -> String {
        match self {
            Self::Success { message } => message.clone().unwrap_or_else(|| "ok".into()),
            Self::Failed { message, kind } => {
                message.clone().unwrap_or_else(|| kind.display_message())
            }
            Self::Cancelled => "cancelled".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfoDto {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionReady {
        modules: Vec<ModuleInfoDto>,
    },
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
        actions: Vec<ActionDescriptorDto>,
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

    #[test]
    fn failed_outcome_preserves_failure_kind_tag() {
        let outcome = ActionOutcomeDto::failed(FailureKind::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Enable AX".into(),
        });
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["failed"]["kind"], "permission_required");
        assert_eq!(json["failed"]["capability"], "accessibility");
        assert!(json["failed"]["message"]
            .as_str()
            .unwrap()
            .contains("permission_required"));
    }

    #[test]
    fn search_item_dto_round_trips_action_risk() {
        let dto = SearchItemDto {
            id: "1".into(),
            module_id: "m".into(),
            title: "t".into(),
            subtitle: None,
            kind: "k".into(),
            score: 1.0,
            primary_action_id: "force".into(),
            primary_action_label: "Force".into(),
            primary_action_risk: ActionRisk::Destructive,
            primary_action_confirmation: true,
            secondary_actions: vec![],
        };
        let item = dto.clone().into_domain();
        assert_eq!(item.primary_action.risk, ActionRisk::Destructive);
        assert!(item.primary_action.confirmation);
        let again = SearchItemDto::from(&item);
        assert_eq!(again.primary_action_risk, ActionRisk::Destructive);
    }
}
