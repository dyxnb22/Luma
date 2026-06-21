import Foundation
import LumaCore

public actor WindowLayoutsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .windowLayouts,
        displayName: "Window Layouts",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(40)
    )

    private let commands = [
        ("left-half", "Move Window Left Half"),
        ("right-half", "Move Window Right Half"),
        ("maximize", "Maximize Window"),
        ("center", "Center Window")
    ]

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard query.normalized.hasPrefix("win ") || query.normalized.hasPrefix("layout ") || query.normalized.contains("left") || query.normalized.contains("right") || query.normalized.contains("max") || query.normalized.contains("center") else {
            return ModuleResult(items: [])
        }

        let items = commands.map { key, title in
            let id = ResultID(module: Self.manifest.identifier, key: key)
            return ResultItem(
                id: id,
                title: title,
                titleAttributed: AttributedString(title),
                subtitle: "Window Layout",
                icon: .symbol("rectangle.split.2x1"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: key),
                    title: title,
                    kind: .applyWindowLayout(key)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        }
        return ModuleResult(items: items)
    }
}
