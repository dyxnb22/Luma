import Foundation
import LumaCore
import LumaServices

public actor CommandsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .commands,
        displayName: "Commands",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 4,
        queryTimeout: .milliseconds(20)
    )

    private static let runnableBuiltIns = ["open-settings", "reload-modules", "quit"]

    private let store: CommandsStore
    private let notesConfigStore: NotesRootConfigStore
    private var cachedCommands: [ScriptCommand] = []
    private var scriptRunner: any ScriptRunnerClient = NoopScriptRunnerClient()
    private var currentProject: any CurrentProjectClient = NoopCurrentProjectClient()
    private var selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient()
    private var reminders: any RemindersClient = NoopRemindersClient()
    private var menuBarTree: any MenuBarTreeClient = NoopMenuBarTreeClient()

    public init(
        store: CommandsStore = CommandsStore(),
        notesConfigStore: NotesRootConfigStore = NotesRootConfigStore()
    ) {
        self.store = store
        self.notesConfigStore = notesConfigStore
    }

    public func warmup(_ context: ModuleContext) async {
        await store.reload()
        cachedCommands = await store.current().commands
        scriptRunner = context.platform.scriptRunner
        currentProject = context.platform.currentProject
        selectionSnapshot = context.platform.selectionSnapshot
        reminders = context.platform.reminders
        menuBarTree = context.platform.menuBarTree
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) {
            return await handlePayload(payload, parsedCommand: query.command, context: context)
        }
        return await handleGlobal(query.normalized, context: context)
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        if case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier {
            if let key = String(data: payload, encoding: .utf8),
               Self.runnableBuiltIns.contains(key) || key == "settings" {
                try await runBuiltIn(key == "settings" ? "open-settings" : key, context: context)
                return
            }
            let decoded = try ModuleActionCoding.decode(CommandsAction.self, from: payload)
            switch decoded {
            case .run(let id):
                guard let command = cachedCommands.first(where: { $0.id == id }) else {
                    throw ModuleError.dataUnavailable
                }
                let expansion = await makeExpansionContext(context: context)
                let cwd = command.cwd.map { TemplateExpander.expand($0, context: expansion) }
                let args = command.args.map { TemplateExpander.expand($0, context: expansion) }
                let request = ScriptRunRequest(
                    title: command.title,
                    executable: command.exec,
                    arguments: args,
                    workingDirectory: cwd,
                    timeoutSeconds: command.timeoutSec
                )
                let runner = scriptRunner
                Task {
                    _ = await runner.run(request)
                }
            case .revealConfig:
                let url = await store.configFileURL()
                await context.platform.workspace.revealInFinder(url)
            case .doctor:
                break
            }
            return
        }
        throw ModuleError.unsupportedAction(action.id)
    }

    public func lastWarmupCommandCount() -> Int {
        cachedCommands.count
    }

    public func commandsConfigValid() async -> Bool {
        let url = await store.configFileURL()
        guard let data = try? Data(contentsOf: url) else { return true }
        return (try? JSONDecoder().decode(CommandsConfig.self, from: data)) != nil
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if runnableBuiltIns.contains(lower) || lower == "settings" || lower == "prefs" {
            return ""
        }
        if lower.hasPrefix("settings ") {
            return String(trimmed.dropFirst(9)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("prefs ") {
            return String(trimmed.dropFirst(6)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower == "cmd" || lower == "command" || lower == "commands" {
            return ""
        }
        if lower.hasPrefix("cmd ") {
            return String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    private func handlePayload(_ payload: String, parsedCommand: ParsedCommand?, context: QueryContext) async -> ModuleResult {
        if let parsedCommand, let builtIn = Self.matchBuiltIn(parsedCommand.trigger) {
            if builtIn == "doctor" {
                return await doctorResult(context: context)
            }
            return ModuleResult(items: [self.command(builtIn, title: Self.title(for: builtIn))])
        }
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if trimmed.isEmpty {
            return listCommands(fallbackBuiltIn: "open-settings")
        }
        if let builtIn = Self.matchBuiltInOrDoctor(trimmed) {
            if builtIn == "doctor" {
                return await doctorResult(context: context)
            }
            return ModuleResult(items: [command(builtIn, title: Self.title(for: builtIn))])
        }
        return listCommands(filter: trimmed)
    }

    private func handleGlobal(_ normalized: String, context: QueryContext) async -> ModuleResult {
        if ModuleHelp.isHelpQuery(normalized) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if let builtIn = Self.matchBuiltIn(normalized) {
            return ModuleResult(items: [command(builtIn, title: Self.title(for: builtIn))])
        }
        if normalized.isEmpty {
            return listCommands()
        }
        return listCommands(filter: normalized)
    }

    private func listCommands(filter: String? = nil, fallbackBuiltIn: String? = nil) -> ModuleResult {
        var items: [ResultItem] = []
        if let fallbackBuiltIn {
            items.append(command(fallbackBuiltIn, title: Self.title(for: fallbackBuiltIn)))
        }
        let normalized = filter?.lowercased() ?? ""
        let matches = cachedCommands.filter { command in
            normalized.isEmpty
                || command.title.lowercased().contains(normalized)
                || command.trigger.lowercased().contains(normalized)
                || command.id.lowercased().contains(normalized)
        }
        items.append(contentsOf: matches.map(scriptRow))
        if items.isEmpty {
            items = Self.runnableBuiltIns.map { command($0, title: Self.title(for: $0)) }
        }
        return ModuleResult(items: items)
    }

    private func doctorResult(context: QueryContext) async -> ModuleResult {
        let axTrusted = await context.platform.accessibility.isTrusted()
        let configValid = await commandsConfigValid()
        let menuCache = await menuBarTree.staleMenuItemCountForFrontmost()
        let notesConfig = await notesConfigStore.load()
        let notesRootConfigured = notesConfig.root != nil
        let remindersAuthorization = await reminders.authorization()
        let manifests = BuiltInModules.manifestCatalog()
        let summary = LumaDiagnostics.summarize(
            manifests: manifests,
            context: LumaDoctorContext(
                accessibilityTrusted: axTrusted,
                remindersAuthorization: remindersAuthorization,
                notesRootConfigured: notesRootConfigured,
                enabledModuleCount: manifests.filter(\.defaultEnabled).count,
                totalModuleCount: manifests.count,
                menuItemsCachedCount: menuCache
            )
        )
        var rows = LumaDiagnostics.doctorRows(
            from: summary,
            module: Self.manifest.identifier,
            basePriority: Self.manifest.priority
        )
        rows.insert(ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "doctor.commands"),
            title: "Script commands loaded: \(cachedCommands.count)",
            titleAttributed: AttributedString("Script commands loaded: \(cachedCommands.count)"),
            subtitle: configValid ? "commands.json OK" : "commands.json invalid",
            icon: .symbol("terminal"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "doctor.commands"),
                title: "Commands",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .informational
        ), at: 1)
        return ModuleResult(items: rows)
    }

    private func scriptRow(_ command: ScriptCommand) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(CommandsAction.run(id: command.id))) ?? Data()
        var secondary: [Action] = []
        let revealPayload = (try? ModuleActionCoding.encode(CommandsAction.revealConfig)) ?? Data()
        secondary.append(Action(
            id: ActionID(module: Self.manifest.identifier, key: "reveal.\(command.id)"),
            title: "Reveal Config",
            kind: .custom(payload: revealPayload, handler: Self.manifest.identifier)
        ))
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "script.\(command.id)"),
            title: command.title,
            titleAttributed: AttributedString(command.title),
            subtitle: command.trigger,
            icon: .symbol("terminal"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "run.\(command.id)"),
                title: "Run \(command.title)",
                kind: .custom(payload: payload, handler: Self.manifest.identifier),
                runsOn: .background
            ),
            secondaryActions: secondary,
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func makeExpansionContext(context: ActionContext) async -> SnippetExpansionContext {
        let project = await context.platform.currentProject.snapshot()
        let selection = await context.platform.selectionSnapshot.snapshot()
        let clipboard = await context.platform.pasteboard.readString()
        return SnippetExpansionContext.from(project: project, clipboardText: clipboard, selectionText: selection)
    }

    private func runBuiltIn(_ key: String, context: ActionContext) async throws {
        switch key {
        case "open-settings":
            await context.host.openSettings()
        case "reload-modules":
            await context.host.reloadModules()
        case "quit":
            await context.host.quitHost()
        default:
            throw ModuleError.unsupportedAction(ActionID(module: Self.manifest.identifier, key: key))
        }
    }

    private static func matchBuiltIn(_ text: String) -> String? {
        let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if normalized == "settings" || normalized == "prefs" { return "open-settings" }
        return runnableBuiltIns.first { $0 == normalized }
    }

    private static func matchBuiltInOrDoctor(_ text: String) -> String? {
        let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if normalized == "doctor" { return "doctor" }
        return matchBuiltIn(text)
    }

    private static func title(for key: String) -> String {
        switch key {
        case "open-settings": "Open Settings"
        case "reload-modules": "Reload Modules"
        case "quit": "Quit Luma"
        case "doctor": "Global Doctor"
        default: key
        }
    }

    private func command(_ key: String, title: String) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: key)
        let payload: Data
        if key == "doctor" {
            payload = (try? ModuleActionCoding.encode(CommandsAction.doctor)) ?? Data(key.utf8)
        } else {
            payload = Data(key.utf8)
        }
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: "Command",
            icon: .symbol("command"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: key),
                title: title,
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
