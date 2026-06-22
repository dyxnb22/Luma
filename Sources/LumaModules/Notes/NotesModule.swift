import Foundation
import LumaCore
import LumaServices

public actor NotesModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .notes,
        displayName: "Notes",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(40)
    )

    private let index: NotesTreeIndex
    private let configStore: NotesRootConfigStore
    private var fsEvents: FSEventsService?
    private var watchTask: Task<Void, Never>?
    private var watchRoot: URL?
    private var rootPath: String?

    public init() {
        index = NotesTreeIndex()
        configStore = NotesRootConfigStore()
    }

    public init(index: NotesTreeIndex, config: NotesRootConfigStore) {
        self.index = index
        configStore = config
    }

    public func warmup(_ context: ModuleContext) async {
        if let service = context.fileSystem as? FSEventsService {
            fsEvents = service
        } else {
            fsEvents = FSEventsService()
        }
        await reloadFromConfig()
    }

    public func teardown() async {
        watchTask?.cancel()
        watchTask = nil
        if let watchRoot, let fsEvents {
            await fsEvents.stop(root: watchRoot)
        }
        self.watchRoot = nil
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let searchText = query.raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !searchText.isEmpty else { return ModuleResult(items: []) }

        if ModuleHelp.isHelpQuery(searchText) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let lower = searchText.lowercased()
        if lower.hasPrefix("backlinks ") {
            let target = String(searchText.dropFirst("backlinks ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !target.isEmpty else { return ModuleResult(items: []) }
            let matches = await findBacklinks(to: target, limit: 12)
            return ModuleResult(items: matches.map(result(for:)))
        }

        let matches = await index.search(fuzzy: searchText, limit: 10)
        return ModuleResult(items: matches.map(result(for:)))
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind else {
            throw ModuleError.unsupportedAction(action.id)
        }
        guard handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(NotesAction.self, from: payload)
        switch decoded {
        case .open(let path):
            let url = URL(fileURLWithPath: path)
            await recordRecent(path: path)
            await MainActor.run {
                NotesTypora.open(url)
            }
        }
    }

    public func recordOpenedNote(path: String) async {
        await recordRecent(path: path)
    }

    private func recordRecent(path: String) async {
        var config = await configStore.load()
        config.recent.removeAll { $0 == path }
        config.recent.insert(path, at: 0)
        if config.recent.count > 5 {
            config.recent = Array(config.recent.prefix(5))
        }
        try? await configStore.save(config)
    }

    public func recentNotePaths() async -> [String] {
        await configStore.load().recent
    }

    public func loadConfig() async -> NotesRootConfig {
        await configStore.load()
    }

    public func saveConfig(_ config: NotesRootConfig) async throws {
        try await configStore.save(config)
    }

    public func snapshot() async -> NotesNode? {
        await index.snapshot()
    }

    public func reloadFromConfig() async {
        watchTask?.cancel()
        watchTask = nil
        if let watchRoot, let fsEvents {
            await fsEvents.stop(root: watchRoot)
        }
        self.watchRoot = nil

        let config = await configStore.load()
        rootPath = config.root?.path
        guard let root = config.root else {
            await index.setRoot(nil)
            return
        }

        await index.setRoot(root)
        await index.warmup()
        await startWatching(root: root)
    }

    public func treeIndex() -> NotesTreeIndex {
        index
    }

    private func startWatching(root: URL) async {
        guard let fsEvents else { return }
        watchRoot = root
        let stream = await fsEvents.watch(root: root)
        watchTask = Task { [index] in
            for await batch in stream {
                if Task.isCancelled { break }
                await index.rebuild(after: batch)
            }
        }
    }

    private func findBacklinks(to target: String, limit: Int) async -> [NotesNode] {
        guard let tree = await index.snapshot() else { return [] }
        let needle = "[[\(target)]]"
        let needleLower = needle.lowercased()
        let notes = flattenNotes(tree)
        var hits: [NotesNode] = []
        for note in notes {
            guard let data = try? String(contentsOfFile: note.path, encoding: .utf8) else { continue }
            if data.lowercased().contains(needleLower) {
                hits.append(note)
            }
            if hits.count >= limit { break }
        }
        return hits
    }

    private func flattenNotes(_ node: NotesNode) -> [NotesNode] {
        var results: [NotesNode] = []
        if node.kind == .note { results.append(node) }
        for child in node.children {
            results.append(contentsOf: flattenNotes(child))
        }
        return results
    }

    private func result(for node: NotesNode) -> ResultItem {
        let parentPath = URL(fileURLWithPath: node.path).deletingLastPathComponent().path
        let subtitle: String
        if let rootPath, parentPath.hasPrefix(rootPath) {
            let relative = parentPath.dropFirst(rootPath.count).trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            subtitle = relative.isEmpty ? "/" : relative
        } else {
            subtitle = parentPath
        }

        let id = ResultID(module: Self.manifest.identifier, key: node.path)
        let payload = (try? ModuleActionCoding.encode(NotesAction.open(path: node.path))) ?? Data()
        return ResultItem(
            id: id,
            title: node.name,
            titleAttributed: AttributedString(node.name),
            subtitle: subtitle,
            icon: .symbol("note.text"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(node.path)"),
                title: "Open in Typora",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
