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
    private var cachedDueTodayCount: Int?
    private var searchIndex: [WordSearchRow] = []
    private var searchResultCache: [String: [WordSearchRow]] = [:]
    private var dataChangeTask: Task<Void, Never>?
    private var dueCacheRefreshTask: Task<Void, Never>?

    internal private(set) var dueTodayCountQueryCount = 0

    internal func isDueCachePopulatedForTesting() -> Bool {
        cachedDueAt != nil
    }

    internal func isDueCacheRefreshInFlightForTesting() -> Bool {
        dueCacheRefreshTask != nil
    }

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
        dueCacheRefreshTask?.cancel()
        dueCacheRefreshTask = nil
    }

    public func storeDueTodayCount() async -> Int {
        dueTodayCountQueryCount += 1
        return (try? await store.dueTodayCount()) ?? 0
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
            let dueCount = cachedDueTodayCount ?? 0
            return ModuleResult(items: [reviewStartRow(dueCount: dueCount)])
        }

        if searchText.isEmpty {
            if let due = freshCachedDueList() {
                return ModuleResult(items: due.map(wordResult))
            }
            scheduleDueCacheRefreshIfNeeded()
            LauncherPerfCounters.increment(.moduleHandleCold)
            return ModuleResult(items: [warmingDueListRow()])
        }

        let cacheKey = searchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if let cached = searchResultCache[cacheKey] {
            return ModuleResult(items: cached.map(wordResult))
        }

        let matches = Self.searchInMemory(searchIndex, query: searchText, limit: 10)
        searchResultCache[cacheKey] = matches
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
        cachedDueTodayCount = nil
    }

    private func freshCachedDueList() -> [WordEntry]? {
        let now = Date()
        guard let cachedDueAt, now.timeIntervalSince(cachedDueAt) < CacheTTL.dueListSeconds else {
            return nil
        }
        return cachedDue
    }

    private func scheduleDueCacheRefreshIfNeeded() {
        guard dueCacheRefreshTask == nil else { return }
        dueCacheRefreshTask = Task {
            defer { dueCacheRefreshTask = nil }
            guard !Task.isCancelled else { return }
            _ = await refreshDueCache(force: false)
        }
    }

    private func warmingDueListRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "warming"),
            title: "Loading due words…",
            titleAttributed: AttributedString("Loading due words…"),
            subtitle: "Refreshing wordbook",
            icon: .symbol("arrow.clockwise"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "noop"),
                title: "Loading",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .informational
        )
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
        guard !Task.isCancelled else { return cachedDue }
        let due = (try? await store.dueWords(limit: 8)) ?? []
        guard !Task.isCancelled else { return cachedDue }
        cachedDue = due
        cachedDueAt = now
        cachedDueTodayCount = (try? await store.dueTodayCount()) ?? 0
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
        )
    }

    private func wordResult(_ word: WordEntry) -> ResultItem {
        wordResult(
            id: word.id,
            term: word.term,
            phonetic: word.phonetic,
            meaning: word.meaning,
            example: word.example,
            category: word.category
        )
    }

    private func wordResult(_ row: WordSearchRow) -> ResultItem {
        wordResult(
            id: row.id,
            term: row.term,
            phonetic: row.phonetic,
            meaning: row.meaning,
            example: row.example,
            category: row.category
        )
    }

    private func wordResult(
        id: Int64,
        term: String,
        phonetic: String,
        meaning: String,
        example: String,
        category: String
    ) -> ResultItem {
        let resultID = ResultID(module: Self.manifest.identifier, key: String(id))
        let subtitle: String
        if category.isEmpty {
            subtitle = meaning
        } else {
            subtitle = "\(meaning) · \(category)"
        }
        return ResultItem(
            id: resultID,
            title: term,
            titleAttributed: AttributedString(term),
            subtitle: subtitle,
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(id)"),
                title: "Copy Meaning",
                kind: .copyToPasteboard("\(term) \(phonetic)\n\(meaning)\n\(example)")
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func reloadSearchIndex() async {
        searchIndex = (try? await store.searchCorpus(limit: 50_000)) ?? []
        searchResultCache.removeAll(keepingCapacity: true)
    }

    private static func searchInMemory(_ index: [WordSearchRow], query: String, limit: Int) -> [WordSearchRow] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !trimmed.isEmpty else { return [] }
        let tokens = trimmed.split(whereSeparator: \.isWhitespace).map(String.init)
        var matches = index.filter { row in
            tokens.allSatisfy { row.haystack.contains($0) }
        }
        if matches.isEmpty {
            matches = index.filter { $0.haystack.contains(trimmed) }
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
