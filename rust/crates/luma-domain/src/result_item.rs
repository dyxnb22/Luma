use crate::id::{ActionId, ModuleId, ResultId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    #[default]
    Safe,
    Confirm,
    Destructive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: ActionId,
    pub label: String,
    pub risk: ActionRisk,
    pub confirmation: bool,
}

impl ActionDescriptor {
    /// True when the TUI must confirm before ExecuteAction, or the engine must reject
    /// an unconfirmed request (`confirmation` flag or non-Safe risk).
    pub fn needs_confirmation(&self) -> bool {
        action_needs_confirmation(self.confirmation, &self.risk)
    }
}

/// Shared confirm predicate for domain actions and protocol DTOs.
pub fn action_needs_confirmation(confirmation: bool, risk: &ActionRisk) -> bool {
    confirmation || !matches!(risk, ActionRisk::Safe)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchItem {
    pub id: ResultId,
    pub module_id: ModuleId,
    pub title: String,
    pub subtitle: Option<String>,
    pub kind: String,
    pub score: f64,
    pub primary_action: ActionDescriptor,
    pub secondary_actions: Vec<ActionDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_intent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_payload: Option<serde_json::Value>,
}
