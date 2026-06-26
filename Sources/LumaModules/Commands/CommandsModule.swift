import Foundation
import LumaCore

public actor CommandsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .commands,
        displayName: "Commands",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 4,
        queryTimeout: .milliseconds(20)
    )

    private static let builtInKeys = ["open-settings", "reload-modules", "quit"]

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) {
            return handlePayload(payload, parsedCommand: query.command)
        }
        return handleGlobal(query.normalized)
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        guard let key = String(data: payload, encoding: .utf8) else {
            throw ModuleError.unsupportedAction(action.id)
        }
        try await runBuiltIn(key, context: context)
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if builtInKeys.contains(lower) || lower == "settings" || lower == "prefs" {
            return ""
        }
        if lower.hasPrefix("settings ") {
            return String(trimmed.dropFirst(9)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("prefs ") {
            return String(trimmed.dropFirst(6)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    private func handlePayload(_ payload: String, parsedCommand: ParsedCommand?) -> ModuleResult {
        if let parsedCommand, let builtIn = Self.matchBuiltIn(parsedCommand.trigger) {
            return ModuleResult(items: [self.command(builtIn, title: Self.title(for: builtIn))])
        }
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if trimmed.isEmpty {
            return ModuleResult(items: [command("open-settings", title: "Open Settings")])
        }
        if let builtIn = Self.matchBuiltIn(trimmed) {
            return ModuleResult(items: [command(builtIn, title: Self.title(for: builtIn))])
        }
        return handleGlobal(trimmed)
    }

    private func handleGlobal(_ normalized: String) -> ModuleResult {
        if ModuleHelp.isHelpQuery(normalized) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if let builtIn = Self.matchBuiltIn(normalized) {
            return ModuleResult(items: [command(builtIn, title: Self.title(for: builtIn))])
        }
        let commands = Self.builtInKeys.map { command($0, title: Self.title(for: $0)) }
        let filtered = commands.filter {
            normalized.isEmpty
                || $0.title.lowercased().contains(normalized)
                || $0.id.key.contains(normalized)
        }
        return ModuleResult(items: filtered)
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
        return builtInKeys.first { $0 == normalized }
    }

    private static func title(for key: String) -> String {
        switch key {
        case "open-settings": "Open Settings"
        case "reload-modules": "Reload Modules"
        case "quit": "Quit Luma"
        default: key
        }
    }

    private func command(_ key: String, title: String) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: key)
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: "Command",
            icon: .symbol("command"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: key),
                title: title,
                kind: .custom(payload: Data(key.utf8), handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
