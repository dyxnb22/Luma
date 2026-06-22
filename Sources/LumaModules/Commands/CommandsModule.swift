import Foundation
import LumaCore

public actor CommandsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .commands,
        displayName: "Commands",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 4,
        queryTimeout: .milliseconds(20)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if ModuleHelp.isHelpQuery(query.normalized) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        let commands = [
            command("open-settings", title: "Open Settings"),
            command("reload-modules", title: "Reload Modules"),
            command("quit", title: "Quit Luma")
        ]
        let filtered = commands.filter { query.normalized.isEmpty || $0.title.lowercased().contains(query.normalized) }
        return ModuleResult(items: filtered)
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
