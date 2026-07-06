import CryptoKit
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
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        if payload.isEmpty {
            return ModuleResult(items: [openDetailStarterRow()])
        }

        let id = ResultID(module: Self.manifest.identifier, key: Self.resultKey(for: payload))
        let item = ResultItem(
            id: id,
            title: "Translate",
            titleAttributed: AttributedString("Translate"),
            subtitle: payload,
            icon: .symbol("character.bubble"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "translate"),
                title: "Translate Text",
                kind: .translateText(payload)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
        return ModuleResult(items: [item])
    }

    static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "tr" || lower == "translate" {
            return ""
        }
        if lower.hasPrefix("translate ") {
            return String(trimmed.dropFirst("translate ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("tr ") {
            return String(trimmed.dropFirst("tr ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    static func resultKey(for text: String) -> String {
        let digest = SHA256.hash(data: Data(text.utf8))
        return digest.prefix(8).map { String(format: "%02x", $0) }.joined()
    }

    private func openDetailStarterRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "open-detail"),
            title: "Open Translate",
            titleAttributed: AttributedString("Open Translate"),
            subtitle: "Translate text in the launcher panel",
            icon: .symbol("character.bubble"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Translate",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
