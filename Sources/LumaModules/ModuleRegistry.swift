import LumaCore

/// Single registration point for all built-in module bundles.
public enum ModuleRegistry {
    public static let allBundles: [any ModuleBundle.Type] = [
        AppsModuleBundle.self,
        ClipboardModuleBundle.self,
        CommandsModuleBundle.self,
        NotesModuleBundle.self,
        TodoModuleBundle.self,
        TranslateModuleBundle.self,
        WordbookModuleBundle.self,
        SnippetsModuleBundle.self,
        SecretsModuleBundle.self,
        MediaModuleBundle.self,
        WindowLayoutsModuleBundle.self,
        ProjectsModuleBundle.self,
        QuicklinksModuleBundle.self,
        MenuItemsModuleBundle.self,
        KillProcessModuleBundle.self,
        BrowserTabsModuleBundle.self
    ]

    public static let defaultPinnedModuleIDs: Set<ModuleIdentifier> = ModuleWarmupDefaults.defaultPinnedModuleIDs

    public static func bundle(for id: ModuleIdentifier) -> (any ModuleBundle.Type)? {
        allBundles.first { $0.identifier == id }
    }

    public static func makeCommandRegistry() -> CommandRegistry {
        var commands: [CommandDefinition] = []
        var shellCommands: [CommandDefinition] = []
        for bundle in allBundles {
            if bundle.identifier == .commands {
                shellCommands.append(contentsOf: bundle.commands)
            } else {
                commands.append(contentsOf: bundle.commands)
            }
        }
        // Shell commands register last so `quit` resolves to exit Luma, not kill-process.
        commands.append(contentsOf: shellCommands)
        return CommandRegistry(commands)
    }

    public static func moduleDetailMetadata() -> [FeatureCard] {
        allBundles.compactMap { $0.detailMetadata }
    }

    public static func manifestCatalog() -> [ModuleManifest] {
        allBundles.map { $0.manifest }
    }

    public static var hotPathModuleIDs: Set<ModuleIdentifier> {
        Set(allBundles.filter { $0.warmupTier == .hotPath }.map { $0.identifier })
    }

    public static var onDemandModuleIDs: Set<ModuleIdentifier> {
        Set(allBundles.filter { $0.warmupTier == .onDemand }.map { $0.identifier })
    }

    /// Module IDs that participate in global (non-targeted) query fan-out.
    public static var globalSearchModuleIDs: Set<ModuleIdentifier> {
        hotPathModuleIDs
    }

    public static func presentation(for id: ModuleIdentifier) -> ModulePresentation? {
        bundle(for: id)?.presentation
    }

    public static func defaultOffNote(for id: ModuleIdentifier) -> String? {
        switch id {
        case .commands:
            return CommandsModuleBundle.defaultOffNote
        case .browserTabs:
            return BrowserTabsModuleBundle.defaultOffNote
        case .media:
            return MediaModuleBundle.defaultOffNote
        default:
            return nil
        }
    }

    public static func displayName(for id: ModuleIdentifier) -> String {
        bundle(for: id)?.manifest.displayName
            ?? id.rawValue
                .replacingOccurrences(of: "luma.", with: "")
                .replacingOccurrences(of: "-", with: " ")
                .capitalized
    }

    public static func makeAll(overrides: BuiltInModules.Overrides = .init()) -> [any LumaModule] {
        allBundles.map { bundle in
            moduleInstance(for: bundle, overrides: overrides)
        }
    }

    private static func moduleInstance(for bundle: any ModuleBundle.Type, overrides: BuiltInModules.Overrides) -> any LumaModule {
        switch bundle.identifier {
        case .clipboard where overrides.clipboard != nil:
            return overrides.clipboard!
        case .notes where overrides.notes != nil:
            return overrides.notes!
        case .todo where overrides.todo != nil:
            return overrides.todo!
        case .wordbook where overrides.wordbook != nil:
            return overrides.wordbook!
        case .snippets where overrides.snippets != nil:
            return overrides.snippets!
        case .secrets where overrides.secrets != nil:
            return overrides.secrets!
        case .media where overrides.media != nil:
            return overrides.media!
        case .projects where overrides.projects != nil:
            return overrides.projects!
        case .quicklinks where overrides.quicklinks != nil:
            return overrides.quicklinks!
        case .menuItems where overrides.menuItems != nil:
            return overrides.menuItems!
        default:
            return bundle.makeModule()
        }
    }
}
