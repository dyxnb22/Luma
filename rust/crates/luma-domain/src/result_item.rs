use crate::id::{ActionId, ModuleId, ResultId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
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
}
