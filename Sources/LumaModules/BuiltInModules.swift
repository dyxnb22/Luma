import LumaCore

public enum BuiltInModules {
    /// Shared module instances for AppCoordinator detail views and settings callbacks.
    public struct Overrides: Sendable {
        public var clipboard: ClipboardModule?
        public var notes: NotesModule?
        public var todo: TodoModule?
        public var wordbook: WordbookModule?
        public var snippets: SnippetsModule?
        public var secrets: SecretsModule?
        public var media: MediaModule?
        public var projects: ProjectsModule?
        public var quicklinks: QuicklinksModule?

        public init(
            clipboard: ClipboardModule? = nil,
            notes: NotesModule? = nil,
            todo: TodoModule? = nil,
            wordbook: WordbookModule? = nil,
            snippets: SnippetsModule? = nil,
            secrets: SecretsModule? = nil,
            media: MediaModule? = nil,
            projects: ProjectsModule? = nil,
            quicklinks: QuicklinksModule? = nil
        ) {
            self.clipboard = clipboard
            self.notes = notes
            self.todo = todo
            self.wordbook = wordbook
            self.snippets = snippets
            self.secrets = secrets
            self.media = media
            self.projects = projects
            self.quicklinks = quicklinks
        }
    }

    /// Active modules registered at launch and warmed up by `ModuleHost`.
    public static func makeAll(overrides: Overrides = .init()) -> [any LumaModule] {
        [
            AppsModule(),
            overrides.clipboard ?? ClipboardModule(),
            CommandsModule(),
            overrides.notes ?? NotesModule(),
            overrides.todo ?? TodoModule(),
            TranslateModule(),
            overrides.wordbook ?? WordbookModule(),
            overrides.snippets ?? SnippetsModule(),
            overrides.secrets ?? SecretsModule(),
            overrides.media ?? MediaModule(),
            WindowLayoutsModule(),
            overrides.projects ?? ProjectsModule(),
            overrides.quicklinks ?? QuicklinksModule(),
            MenuItemsModule(),
            KillProcessModule(),
            BrowserTabsModule()
        ]
    }

    /// Deferred modules kept in source but excluded from active dashboard, warmup, and default registration.
    public static func makeDeferred() -> [any LumaModule] {
        [WindowsModule()]
    }

    public static let accessibilityDependentModuleIDs: Set<ModuleIdentifier> = [.windows, .snippets, .windowLayouts, .menuItems]

    public static func enabledModulesRequireAccessibility(_ enabled: Set<ModuleIdentifier>) -> Bool {
        !accessibilityDependentModuleIDs.isDisjoint(with: enabled)
    }

    public static func activeModulesRequireAccessibility() -> Bool {
        enabledModulesRequireAccessibility(Set(makeAll().map { type(of: $0).manifest.identifier }))
    }

    /// Single source of truth for built-in module manifests (registration order).
    public static func manifestCatalog() -> [ModuleManifest] {
        [
            AppsModule.manifest,
            ClipboardModule.manifest,
            CommandsModule.manifest,
            NotesModule.manifest,
            TodoModule.manifest,
            TranslateModule.manifest,
            WordbookModule.manifest,
            SnippetsModule.manifest,
            SecretsModule.manifest,
            MediaModule.manifest,
            WindowLayoutsModule.manifest,
            ProjectsModule.manifest,
            QuicklinksModule.manifest,
            MenuItemsModule.manifest,
            KillProcessModule.manifest,
            BrowserTabsModule.manifest
        ]
    }
}
