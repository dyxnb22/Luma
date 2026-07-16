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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchFailure {
    Unavailable { reason: String },
    NotConfigured { hint: String },
    PermissionRequired { capability: String },
    Warming,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStatus {
    Ready,
    Warming,
    Failed(SearchFailure),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiIntent {
    Browse,
    SeedConfig,
    ListIssues,
    SeedAdd,
    OpenPath,
}

impl UiIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Browse => "browse",
            Self::SeedConfig => "seed_config",
            Self::ListIssues => "list_issues",
            Self::SeedAdd => "seed_add",
            Self::OpenPath => "open_path",
        }
    }

    pub fn parse(tag: &str) -> Option<Self> {
        match tag {
            "browse" => Some(Self::Browse),
            "seed_config" | "configure" => Some(Self::SeedConfig),
            "list_issues" => Some(Self::ListIssues),
            "seed_add" => Some(Self::SeedAdd),
            "open_path" => Some(Self::OpenPath),
            _ => None,
        }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_intent: Option<UiIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_payload: Option<serde_json::Value>,
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
            ui_intent: None,
            action_payload: None,
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
            ui_intent: item.ui_intent.as_deref().and_then(UiIntent::parse),
            action_payload: item.action_payload.clone(),
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
            ui_intent: self.ui_intent.map(|i| i.as_str().to_string()),
            action_payload: self.action_payload,
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
    /// TUI-owned: run program with args in the current terminal (no shell).
    InteractiveTerminal {
        program: String,
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        record_alias: Option<String>,
    },
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
            Self::InteractiveTerminal { .. } => "interactive terminal".into(),
        }
    }

    pub fn is_interactive_terminal(&self) -> bool {
        matches!(self, Self::InteractiveTerminal { .. })
    }

    /// TUI status copy (prefer explicit message, else FailureKind::user_message).
    pub fn user_message(&self) -> String {
        match self {
            Self::Success { message } => message.clone().unwrap_or_else(|| "ok".into()),
            Self::Failed { message, kind } => message
                .clone()
                .filter(|m| {
                    !m.starts_with("not_configured:")
                        && !m.starts_with("permission_required")
                        && !m.starts_with("unavailable")
                        && !m.starts_with("invalid_input")
                        && !m.starts_with("security_denied")
                        && !m.starts_with("conflict:")
                        && !m.starts_with("io:")
                        && !m.starts_with("internal")
                })
                .unwrap_or_else(|| kind.user_message()),
            Self::Cancelled => "Cancelled".into(),
            Self::InteractiveTerminal { .. } => "connecting…".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfoDto {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    #[serde(default)]
    pub glyph: Option<String>,
    #[serde(default)]
    pub suggested_query: Option<String>,
    #[serde(default)]
    pub empty_hint: Option<String>,
    #[serde(default)]
    pub supports_browse: bool,
    #[serde(default)]
    pub triggers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordReviewWordDto {
    pub id: i64,
    pub term: String,
    pub phonetic: String,
    pub meaning: String,
    pub example: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordbookStatsDto {
    pub due: i64,
    pub new_count: i64,
    pub wrong: i64,
    pub goal: i64,
    pub reviewed_today: i64,
    pub remaining_goal: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubWindowDto {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubWindowsStatusDto {
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubWindowsDto {
    pub app_name: String,
    pub windows: Vec<HubWindowDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<HubWindowsStatusDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionReady {
        modules: Vec<ModuleInfoDto>,
    },
    HubLoaded {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        windows: Option<HubWindowsDto>,
    },
    SnapshotLoaded {
        items: Vec<SearchItemDto>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        module_states: std::collections::HashMap<String, String>,
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
    PreviewLoaded {
        result_id: String,
        preview_id: u64,
        body: String,
    },
    WordbookReviewLoaded {
        queue: String,
        words: Vec<WordReviewWordDto>,
        stats: WordbookStatsDto,
    },
    WordbookReviewStatsUpdated {
        stats: WordbookStatsDto,
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
            ui_intent: None,
            action_payload: None,
        };
        let item = dto.clone().into_domain();
        assert_eq!(item.primary_action.risk, ActionRisk::Destructive);
        assert!(item.primary_action.confirmation);
        let again = SearchItemDto::from(&item);
        assert_eq!(again.primary_action_risk, ActionRisk::Destructive);
    }
}
