use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    StartSession,
    Search {
        request_id: String,
        query: String,
    },
    CancelSearch {
        request_id: String,
    },
    ListActions {
        result_id: String,
    },
    ExecuteAction {
        operation_id: String,
        result_id: String,
        action_id: String,
        confirmation: bool,
    },
    CancelOperation {
        operation_id: String,
    },
    SetModuleEnabled {
        module_id: String,
        enabled: bool,
    },
    GetSettings,
    UpdateSettings {
        patch: serde_json::Value,
        expected_version: u64,
    },
    LoadPreview {
        result_id: String,
        preview_id: u64,
    },
    LoadHub,
    LoadWordbookReview {
        queue: String,
    },
    RefreshWordbookReviewStats,
    GetSnapshot,
    ShutdownSession,
    RecordRecipeRun {
        recipe_id: String,
        result: luma_domain::RecipeRunOutcome,
        now_unix: i64,
    },
    /// After an interactive SSH/SFTP session ends in the TUI.
    SshSessionEnded {
        alias: String,
        exit_code: i32,
    },
}
