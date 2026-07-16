//! Injected infrastructure ports. Adapters live in platform/storage; modules only see these.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod capability;
mod clipboard_repo;
mod clock;
mod command_recipes_repo;
mod command_runner;
mod fake_recipe_environment;
mod keychain;
mod markdown_watch;
mod memory_repos;
mod notes_repo;
mod notes_workspace;
mod open_path;
mod pasteboard;
mod profile;
mod project_workspace;
mod proxy_core;
mod quicklinks_repo;
mod recipe_environment;
mod records_repo;
mod settings;
mod snippets_repo;
mod speech;
mod ssh_config;
mod ssh_meta_repo;
mod system_proxy;
mod window;
mod wordbook_repo;

pub use accessibility::{AccessibilityError, AccessibilityPort, FakeAccessibility};
pub use apps::{AppEntry, AppLaunchError, AppsCatalogPort};
pub use bounded_file_reader::{
    BoundedUtf8FileReadError, BoundedUtf8FileReaderPort, FakeBoundedUtf8FileReader,
};
pub use capability::{CapabilityPort, FakeCapabilities};
pub use clipboard_repo::{
    looks_secret, ClipboardEntry, ClipboardHistoryRepository, ClipboardRepoError,
};
pub use clock::{ClockError, ClockPort, FixedClock};
pub use command_recipes_repo::{CommandRecipesRepoError, CommandRecipesRepository};
pub use command_runner::{filter_env_output, is_filtered_env_step, FakeCommandRunner};
pub use fake_recipe_environment::FakeRecipeEnvironment;
pub use keychain::{FakeKeychain, KeychainError, KeychainPort, SecretLabel};
pub use markdown_watch::{FakeMarkdownWatcher, MarkdownWatchPort};
pub use memory_repos::{
    FakeSshConfigPort, MemoryClipboardHistory, MemoryNotesIndex, MemoryQuicklinksRepository,
    MemoryRecordsRepository, MemorySnippetsRepository, MemorySshMetaRepository,
    MemoryWordbookRepository,
};
pub use notes_repo::{
    NotesDocument, NotesIndexError, NotesIndexRepository, NotesIssue, NotesLink, NotesScanReport,
    NotesScanStatusView, NotesSearchHit,
};
pub use notes_workspace::{
    FakeNotesWorkspace, NotesDirectoryEntry, NotesDirectoryEntryKind, NotesDirectoryListing,
    NotesWorkspaceError, NotesWorkspacePath, NotesWorkspacePort, NotesWorkspacePreview,
};
pub use open_path::{FakeOpenPath, OpenPathError, OpenPathPort};
pub use pasteboard::{FakePasteboard, PasteboardError, PasteboardPort};
pub use profile::{
    ProfileImportResult, ProfileSource, ProfileStoreError, ProfileStorePort, ProfileSummary,
};
pub use project_workspace::{
    FakeProjectWorkspace, ProjectDirectoryEntry, ProjectDirectoryListing, ProjectOpenScope,
    ProjectWorkspaceError, ProjectWorkspacePort,
};
pub use proxy_core::{
    ExternalControllerStatus, FakeProxyCore, ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode,
    ProxyNode, ProxyPorts, ProxyStatus,
};
pub use quicklinks_repo::{QuicklinkEntry, QuicklinksRepoError, QuicklinksRepository};
pub use recipe_environment::{
    recipe_in_scope, recipe_runnable, resolve_steps, select_best_variant, CommandRunnerPort,
    PathKind, RecipeEnvironmentError, RecipeEnvironmentPort,
};
pub use records_repo::{
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView,
};
pub use settings::{AppSettings, SettingsError, SettingsRepository};
pub use snippets_repo::{SnippetEntry, SnippetsRepoError, SnippetsRepository};
pub use speech::{FakeSpeech, SpeechAccent, SpeechError, SpeechPort};
pub use ssh_config::{
    format_connection_subtitle, sanitize_identity_display, ResolvedSshHost, SshConfigError,
    SshConfigPort, SshConfigState,
};
pub use ssh_meta_repo::{SshHostMeta, SshMetaRepoError, SshMetaRepository};
pub use system_proxy::{
    FakeSystemProxy, SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus,
};
pub use window::{FakeWindowCatalog, WindowCatalogPort, WindowEntry, WindowError};
pub use wordbook_repo::{
    ContentImportReport, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
