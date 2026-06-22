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

        public init(
            clipboard: ClipboardModule? = nil,
            notes: NotesModule? = nil,
            todo: TodoModule? = nil,
            wordbook: WordbookModule? = nil,
            snippets: SnippetsModule? = nil,
            secrets: SecretsModule? = nil,
            media: MediaModule? = nil
        ) {
            self.clipboard = clipboard
            self.notes = notes
            self.todo = todo
            self.wordbook = wordbook
            self.snippets = snippets
            self.secrets = secrets
            self.media = media
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
            EventsModule(),
            TranslateModule(),
            overrides.wordbook ?? WordbookModule(),
            overrides.snippets ?? SnippetsModule(),
            overrides.secrets ?? SecretsModule(),
            overrides.media ?? MediaModule()
        ]
    }

    /// Deferred modules kept in source but excluded from active dashboard, warmup, and default registration.
    public static func makeDeferred() -> [any LumaModule] {
        [
            WindowsModule(),
            CalculatorModule()
        ]
    }

    public static let accessibilityDependentModuleIDs: Set<ModuleIdentifier> = [.windows, .snippets]

    public static func activeModulesRequireAccessibility() -> Bool {
        !accessibilityDependentModuleIDs.isDisjoint(with: Set(makeAll().map { type(of: $0).manifest.identifier }))
    }
}
