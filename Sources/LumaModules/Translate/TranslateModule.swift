import Foundation
import LumaCore

public actor TranslateModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .translate,
        displayName: "Translate",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(60)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard query.normalized.hasPrefix("tr ") || query.normalized.hasPrefix("translate ") else {
            return ModuleResult(items: [])
        }

        let text = Self.translationText(from: query.raw)

        guard !text.isEmpty else { return ModuleResult(items: []) }
        let id = ResultID(module: Self.manifest.identifier, key: text)
        let item = ResultItem(
            id: id,
            title: "Translate",
            titleAttributed: AttributedString("Translate"),
            subtitle: text,
            icon: .symbol("character.bubble"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "translate"),
                title: "Translate Text",
                kind: .translateText(text)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
        return ModuleResult(items: [item])
    }

    private static func translationText(from raw: String) -> String {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lowercased = trimmed.lowercased()
        if lowercased.hasPrefix("translate ") {
            return String(trimmed.dropFirst("translate ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lowercased.hasPrefix("tr ") {
            return String(trimmed.dropFirst("tr ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return trimmed
    }
}
