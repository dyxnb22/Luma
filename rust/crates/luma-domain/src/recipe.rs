use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRisk {
    #[default]
    Safe,
    Confirm,
    Destructive,
}

impl RecipeRisk {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "confirm" => Some(Self::Confirm),
            "destructive" => Some(Self::Destructive),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Confirm => "confirm",
            Self::Destructive => "destructive",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeScope {
    #[default]
    Global,
    CurrentProject,
    LumaRepository,
}

impl RecipeScope {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "global" => Some(Self::Global),
            "current_project" => Some(Self::CurrentProject),
            "luma_repository" => Some(Self::LumaRepository),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::CurrentProject => "current_project",
            Self::LumaRepository => "luma_repository",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandStep {
    pub id: String,
    pub label: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
    #[serde(default)]
    pub continue_on_error: bool,
}

fn default_cwd() -> String {
    "current".into()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeVariant {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires_files: Vec<String>,
    #[serde(default)]
    pub requires_directories: Vec<String>,
    #[serde(default)]
    pub requires_commands: Vec<String>,
    #[serde(default)]
    pub steps: Vec<CommandStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub scope: RecipeScope,
    #[serde(default)]
    pub risk: RecipeRisk,
    #[serde(default)]
    pub variants: Vec<RecipeVariant>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRunOutcome {
    #[default]
    Success,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeMetadata {
    pub favorite: bool,
    pub last_used_at: Option<i64>,
    pub use_count: u64,
    pub last_result: Option<RecipeRunOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedCommandStep {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub continue_on_error: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeRunPlan {
    pub recipe_id: String,
    pub recipe_title: String,
    pub risk: RecipeRisk,
    pub working_dir: PathBuf,
    pub variant_id: String,
    pub variant_description: String,
    pub steps: Vec<ResolvedCommandStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantMatch {
    Matched(RecipeVariant),
    NoMatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigIssue {
    pub location: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecipeCatalog {
    pub recipes: Vec<Recipe>,
    pub issues: Vec<ConfigIssue>,
    pub config_path: Option<PathBuf>,
}

impl RecipeCatalog {
    pub fn recipe_by_id(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRunResult {
    pub step_id: String,
    pub exit_code: Option<i32>,
    pub started: bool,
    pub message: Option<String>,
}
