import Foundation
import LumaCore

public actor WordbookModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .wordbook,
        displayName: "Wordbook",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(40)
    )

    private let store = WordbookStore()

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard query.normalized.hasPrefix("word ") || query.normalized == "words" || query.normalized == "review" else {
            return ModuleResult(items: [])
        }

        let searchText = query.raw
            .replacingOccurrences(of: "words", with: "", options: [.caseInsensitive])
            .replacingOccurrences(of: "word", with: "", options: [.caseInsensitive])
            .replacingOccurrences(of: "review", with: "", options: [.caseInsensitive])
            .trimmingCharacters(in: .whitespacesAndNewlines)

        if !searchText.isEmpty, let matches = try? await store.search(searchText, limit: 10), !matches.isEmpty {
            return ModuleResult(items: matches.map(wordResult))
        }

        if let due = try? await store.dueWords(limit: 10), !due.isEmpty {
            return ModuleResult(items: due.map(wordResult))
        }

        let count = (try? await store.count()) ?? 0
        let id = ResultID(module: Self.manifest.identifier, key: "review")
        let item = ResultItem(
            id: id,
            title: "Review Words",
            titleAttributed: AttributedString("Review Words"),
            subtitle: "\(count) words from wordbot",
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "review"),
                title: "Start Review",
                kind: .custom(payload: Data(), handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
        return ModuleResult(items: [item])
    }

    private func wordResult(_ word: WordEntry) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: String(word.id))
        return ResultItem(
            id: id,
            title: word.term,
            titleAttributed: AttributedString(word.term),
            subtitle: "\(word.meaning) · \(word.category)",
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "speak.\(word.id)"),
                title: "Copy Meaning",
                kind: .copyToPasteboard("\(word.term) \(word.phonetic)\n\(word.meaning)\n\(word.example)")
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
