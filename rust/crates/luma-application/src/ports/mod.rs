//! Injected infrastructure ports. Adapters live in platform/storage; modules only see these.

mod accessibility;
mod apps;
mod bounded_file_reader;
mod capability;
mod clipboard_repo;
mod clock;
mod command_recipes_repo;
mod command_runner;
mod databases;
mod downloads;
mod fake_recipe_environment;
mod git;
mod keychain;
mod memory_repos;
mod network_probe;
mod open_path;
mod packages;
mod pasteboard;
mod profile;
mod project_workspace;
mod proxy_core;
mod quicklinks_repo;
mod recall_repo;
mod recipe_environment;
mod records_repo;
mod renewals_repo;
mod runtime;
mod screen_ocr;
mod settings;
mod shell_history;
mod shortcuts;
mod snippets_repo;
mod speech;
mod ssh_config;
mod ssh_meta_repo;
mod system_proxy;
mod system_settings;
mod timers_repo;
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
pub use clock::{ClockError, ClockPort, ControllableClock, FixedClock};
pub use command_recipes_repo::{CommandRecipesRepoError, CommandRecipesRepository};
pub use command_runner::{filter_env_output, is_filtered_env_step, FakeCommandRunner};
pub use databases::{
    postgres_client_args, validate_postgres_metadata, DatabaseClientPlan, DatabasePlatformError,
    DatabasePlatformPort, DatabasePortal, DatabasePortalTarget, DatabasePortalsRepoError,
    DatabasePortalsRepository, DatabaseSchemaObject, FakeDatabasePlatform,
    MemoryDatabasePortalsRepository, NewDatabasePortal, MAX_DATABASE_PORTALS,
    MAX_DATABASE_SCHEMA_BYTES, MAX_DATABASE_SCHEMA_OBJECTS,
};
pub use downloads::{
    DownloadCategory, DownloadEntry, DownloadsError, DownloadsFilter, DownloadsPort, FakeDownloads,
    MAX_DOWNLOAD_ENTRIES,
};
pub use fake_recipe_environment::FakeRecipeEnvironment;
pub use git::{
    FakeGitRepository, GitBranch, GitCommit, GitDiff, GitError, GitFile, GitProjectRoot,
    GitRepositoryPort, GitRepositoryState,
};
pub use keychain::{FakeKeychain, KeychainError, KeychainPort, SecretLabel};
pub use memory_repos::{
    FakeSshConfigPort, MemoryClipboardHistory, MemoryQuicklinksRepository, MemoryRecordsRepository,
    MemorySnippetsRepository, MemorySshMetaRepository, MemoryTimersRepository,
    MemoryWordbookRepository,
};
pub use network_probe::{
    FakeNetworkProbe, NetworkProbePort, NetworkProbeState, NetworkProbeStep,
    UnavailableNetworkProbe,
};
pub use open_path::{FakeOpenPath, OpenPathError, OpenPathPort};
pub use packages::{
    mutation_args, validate_mutation_state, FakePackageManager, PackageError, PackageKind,
    PackageManagerPort, PackageMutation, PackageMutationPlan, PackageQuery, PackageRecord,
    MAX_PACKAGE_OUTPUT_BYTES, MAX_PACKAGE_RESULTS,
};
pub use pasteboard::{FakePasteboard, PasteboardError, PasteboardPort, PasteboardSnapshot};
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
pub use recall_repo::{RecallObject, RecallRepoError, RecallRepository, MAX_RECALL_TITLE_CHARS};
pub use recipe_environment::{
    recipe_in_scope, recipe_runnable, resolve_steps, select_best_variant, CommandRunnerPort,
    PathKind, RecipeEnvironmentError, RecipeEnvironmentPort, RecipeStdioMode,
};
pub use records_repo::{
    RecordCategory, RecordEntry, RecordImportPreviewView, RecordImportReportView, RecordsRepoError,
    RecordsRepository, RecordsStatsView,
};
pub use renewals_repo::{
    MemoryRenewalsRepository, NewRenewal, RenewalEntry, RenewalPaidUpdate, RenewalsRepoError,
    RenewalsRepository, MAX_RENEWALS,
};
pub use runtime::{FakeRuntimePort, RuntimeError, RuntimeListener, RuntimePort};
pub use screen_ocr::{FakeScreenOcr, ScreenOcrError, ScreenOcrPort, MAX_OCR_TEXT_BYTES};
pub use settings::{AppSettings, SettingsError, SettingsRepository};
pub use shell_history::{
    FakeShellHistory, ShellHistoryEntry, ShellHistoryError, ShellHistoryPort, ShellHistorySnapshot,
    MAX_SHELL_HISTORY_BYTES, MAX_SHELL_HISTORY_COMMAND_BYTES, MAX_SHELL_HISTORY_ENTRIES,
};
pub use shortcuts::{
    FakeShortcuts, ShortcutEntry, ShortcutRunPlan, ShortcutsError, ShortcutsPort,
    MAX_SHORTCUT_OUTPUT_BYTES, MAX_SHORTCUT_RESULTS,
};
pub use snippets_repo::{SnippetEntry, SnippetsRepoError, SnippetsRepository};
pub use speech::{FakeSpeech, SpeechAccent, SpeechError, SpeechPort};
pub use ssh_config::{
    format_connection_subtitle, sanitize_identity_display, ssh_password_account, ResolvedSshHost,
    SshConfigError, SshConfigPort, SshConfigState, SSH_ASKPASS_ACCOUNT_ENV,
    SSH_PASSWORD_ACCOUNT_PREFIX,
};
pub use ssh_meta_repo::{SshHostMeta, SshMetaRepoError, SshMetaRepository};
pub use system_proxy::{
    FakeSystemProxy, SystemProxyError, SystemProxyPort, SystemProxySetting, SystemProxyStatus,
};
pub use system_settings::{
    FakeSystemSettings, SystemSettingsError, SystemSettingsPane, SystemSettingsPort,
};
pub use timers_repo::{TimerEntry, TimersRepoError, TimersRepository};
pub use window::{FakeWindowCatalog, WindowCatalogPort, WindowEntry, WindowError};
pub use wordbook_repo::{
    ContentImportReport, WordContentInput, WordEntry, WordbookRepoError, WordbookRepository,
    WordbookStatsView,
};
