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
    private var searchIndex: [WordEntry] = []
    private var dataChangeTask: Task<Void, Never>?

    public init(store: WordbookStore = WordbookStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        await refreshDueCache(force: true)
        await reloadSearchIndex()
        dataChangeTask?.cancel()
        dataChangeTask = Task { [weak self] in
            for await _ in WordbookStoreChangeHub.dataChanges() {
                guard let self else { return }
                await self.invalidateDueCache()
                await self.reloadSearchIndex()
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
            return ModuleResult(items: [reviewStartRow(dueCount: await storeDueTodayCount())])
        }

        if searchText.isEmpty {
            let due = await cachedDueList()
            return ModuleResult(items: due.map(wordResult))
        }

        let matches = Self.searchInMemory(searchIndex, query: searchText, limit: 10)
        if !matches.isEmpty {
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

    private func reviewStartRow(dueCount: Int) -> ResultItem {
        let subtitle: String
        if dueCount == 0 {
            subtitle = "No words due — open Wordbook to browse or import"
        } else if dueCount == 1 {
            subtitle = "1 word due today"
        } else {
            subtitle = "\(dueCount) words due today"
        }
        let payload = (try? ModuleActionCoding.encode(WordbookAction.review)) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "review"),
            title: "Start Review",
            titleAttributed: AttributedString("Start Review"),
            subtitle: subtitle,
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "review"),
                title: "Start Review",
                kind: .openModuleDetail(.wordbook, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 3),
            rowKind: .starter
        )
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

    private func reloadSearchIndex() async {
        searchIndex = (try? await store.allWords(limit: 50_000, offset: 0)) ?? []
    }

    private static func searchInMemory(_ index: [WordEntry], query: String, limit: Int) -> [WordEntry] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !trimmed.isEmpty else { return [] }
        let tokens = trimmed.split(whereSeparator: \.isWhitespace).map(String.init)
        var matches = index.filter { entry in
            let haystack = [entry.term, entry.meaning, entry.example, entry.category, entry.phonetic]
                .joined(separator: " ")
                .lowercased()
            return tokens.allSatisfy { haystack.contains($0) }
        }
        if matches.isEmpty {
            matches = index.filter { entry in
                let haystack = [entry.term, entry.meaning, entry.example, entry.category]
                    .joined(separator: " ")
                    .lowercased()
                return haystack.contains(trimmed)
            }
        }
        return Array(matches.prefix(limit))
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
