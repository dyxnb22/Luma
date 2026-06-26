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

    private let store: WordbookStore
    private var cachedDue: [WordEntry] = []
    private var cachedDueAt: Date?
    private var dataChangeTask: Task<Void, Never>?

    public init(store: WordbookStore = WordbookStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        await refreshDueCache(force: true)
        dataChangeTask?.cancel()
        dataChangeTask = Task { [weak self] in
            for await _ in WordbookStoreChangeHub.dataChanges() {
                guard let self else { return }
                await self.invalidateDueCache()
            }
        }
    }

    public func teardown() async {
        dataChangeTask?.cancel()
        dataChangeTask = nil
    }

    public func storeDueTodayCount() async -> Int {
        (try? await store.dueTodayCount()) ?? 0
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let searchText = payload

        if searchText.compare("review", options: .caseInsensitive) == .orderedSame {
            return ModuleResult(items: [])
        }

        if searchText.isEmpty {
            let due = await cachedDueList()
            return ModuleResult(items: due.map(wordResult))
        }

        if let matches = try? await store.search(searchText, limit: 10), !matches.isEmpty {
            return ModuleResult(items: matches.map(wordResult))
        }

        return ModuleResult(items: [])
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        throw ModuleError.unsupportedAction(action.id)
    }

    public func invalidateDueCache() async {
        cachedDue = []
        cachedDueAt = nil
    }

    private func cachedDueList() async -> [WordEntry] {
        let now = Date()
        if let cachedDueAt, now.timeIntervalSince(cachedDueAt) < CacheTTL.dueListSeconds, !cachedDue.isEmpty {
            return cachedDue
        }
        return await refreshDueCache(force: false)
    }

    @discardableResult
    private func refreshDueCache(force: Bool) async -> [WordEntry] {
        let now = Date()
        if !force, let cachedDueAt, now.timeIntervalSince(cachedDueAt) < CacheTTL.dueListSeconds {
            return cachedDue
        }
        let due = (try? await store.dueWords(limit: 8)) ?? []
        cachedDue = due
        cachedDueAt = now
        return due
    }

    private func wordResult(_ word: WordEntry) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: String(word.id))
        let subtitle: String
        if word.category.isEmpty {
            subtitle = word.meaning
        } else {
            subtitle = "\(word.meaning) · \(word.category)"
        }
        return ResultItem(
            id: id,
            title: word.term,
            titleAttributed: AttributedString(word.term),
            subtitle: subtitle,
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(word.id)"),
                title: "Copy Meaning",
                kind: .copyToPasteboard("\(word.term) \(word.phonetic)\n\(word.meaning)\n\(word.example)")
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "word" || lower == "wb" {
            return ""
        }
        if lower.hasPrefix("wb ") {
            return String(trimmed.dropFirst(3)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("word ") {
            return String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
