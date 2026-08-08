//! Injected infrastructure ports. Adapters live in platform/storage; modules only see these.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod capability;
mod clipboard_repo;
mod command_recipes_repo;
mod command_runner;
mod fake_recipe_environment;
mod git;
mod memory_repos;
mod open_path;
mod pasteboard;
mod project_workspace;
mod recall_repo;
mod recipe_environment;
mod records_repo;
mod runtime;
mod settings;
mod speech;
mod system_settings;
mod window;
mod wordbook_repo;

pub use accessibility::{
    frontmost_matches_paste_target, AccessibilityError, AccessibilityPort, FakeAccessibility,
};
pub use apps::{AppEntry, AppLaunchError, AppsCatalogPort};
pub use bounded_file_reader::{
    BoundedUtf8FileReadError, BoundedUtf8FileReaderPort, FakeBoundedUtf8FileReader,
};
pub use capability::{CapabilityPort, FakeCapabilities};
pub use clipboard_repo::{
    looks_secret, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError,
};
pub use command_recipes_repo::{CommandRecipesRepoError, CommandRecipesRepository};
pub use command_runner::{filter_env_output, is_filtered_env_step, FakeCommandRunner};
pub use fake_recipe_environment::FakeRecipeEnvironment;
pub use git::{
    FakeGitRepository, GitBranch, GitCommit, GitDiff, GitError, GitFile, GitProjectRoot,
    GitRepositoryPort, GitRepositoryState,
};
pub use memory_repos::{MemoryClipboardHistory, MemoryRecordsRepository, MemoryWordbookRepository};
pub use open_path::{FakeOpenPath, OpenPathError, OpenPathPort};
pub use pasteboard::{FakePasteboard, PasteboardError, PasteboardPort, PasteboardSnapshot};
pub use project_workspace::{
    FakeProjectWorkspace, ProjectDirectoryEntry, ProjectDirectoryListing, ProjectOpenScope,
    ProjectWorkspaceError, ProjectWorkspacePort,
};
pub use recall_repo::{RecallObject, RecallRepoError, RecallRepository, MAX_RECALL_TITLE_CHARS};
pub use recipe_environment::{
    recipe_in_scope, recipe_runnable, resolve_steps, select_best_variant, CommandRunnerPort,
    PathKind, RecipeEnvironmentError, RecipeEnvironmentPort, RecipeStdioMode,
};
pub use records_repo::{
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView,
};
pub use runtime::{FakeRuntimePort, RuntimeError, RuntimeListener, RuntimePort};
pub use settings::{AppSettings, SettingsError, SettingsRepository};
pub use speech::{FakeSpeech, SpeechAccent, SpeechError, SpeechPort};
pub use system_settings::{
    FakeSystemSettings, SystemSettingsError, SystemSettingsPane, SystemSettingsPort,
};
pub use window::{FakeWindowCatalog, WindowCatalogPort, WindowEntry, WindowError};
pub use wordbook_repo::{
    ContentImportReport, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
