import Foundation
import LumaCore

public actor MediaModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .media,
        displayName: "Media",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(30)
    )

    private let store: MediaStore
    private var cachedItems: [MediaItem] = []

    public init(store: MediaStore = MediaStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        cachedItems = await store.all()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized
        guard normalized == "m" || normalized == "media" || normalized.hasPrefix("m ") || normalized.hasPrefix("media ") else {
            return ModuleResult(items: [])
        }

        let payload: String
        if normalized == "m" || normalized == "media" {
            payload = ""
        } else if normalized.hasPrefix("m ") {
            payload = String(query.raw.dropFirst(2)).trimmingCharacters(in: .whitespacesAndNewlines)
        } else {
            payload = String(query.raw.dropFirst(6)).trimmingCharacters(in: .whitespacesAndNewlines)
        }

        if payload.isEmpty {
            return await emptyQueryResult()
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        if payload.lowercased() == "log" {
            return ModuleResult(items: [manageLogResult()])
        }

        let parsed = MediaParser.parse(payload)
        switch parsed.mode {
        case .capture:
            return ModuleResult(items: [captureResult(parsed)])
        case .search:
            let matches = MediaIndex.search(cachedItems, query: parsed.title, limit: 8)
            if !matches.isEmpty {
                return ModuleResult(items: matches.map { itemResult($0.item) })
            }
            if parsed.title.isEmpty {
                return ModuleResult(items: [])
            }
            let partial = MediaParser.Result(
                mode: .capture(partial: true),
                title: parsed.title,
                category: nil,
                rating: nil,
                status: .done,
                hadDSLToken: false
            )
            return ModuleResult(items: [captureResult(partial)])
        }
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(MediaAction.self, from: payload)

        switch decoded {
        case .openDetail:
            await MainActor.run { LauncherBridge.openMediaDetail?() }
        case .edit(let id):
            guard let item = cachedItems.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            await MainActor.run {
                LauncherBridge.pendingMediaEditorDraft = MediaEditorDraft(item: item)
                LauncherBridge.openMediaDetail?()
            }
        case .editDraft(let draft):
            await MainActor.run {
                LauncherBridge.pendingMediaEditorDraft = draft
                LauncherBridge.openMediaDetail?()
            }
        case .capture(let draft):
            _ = try await store.add(from: draft)
            await refreshCache()
            await MainActor.run { LauncherBridge.reloadMediaDetail?() }
        case .copy(let id):
            guard let item = cachedItems.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            let summary = Self.summaryLine(for: item)
            await context.pasteboard.write(summary)
        }
    }

    public func allItems() async -> [MediaItem] {
        await store.all()
    }

    public func add(from draft: MediaEditorDraft) async throws -> MediaItem {
        let item = try await store.add(from: draft)
        await refreshCache()
        return item
    }

    public func update(from draft: MediaEditorDraft) async throws -> MediaItem {
        let item = try await store.update(from: draft)
        await refreshCache()
        return item
    }

    public func delete(id: UUID) async throws {
        try await store.delete(id: id)
        await refreshCache()
    }

    public func exportCSV() async throws -> URL {
        let items = await store.all()
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withFullDate]
        var lines = ["id,title,category,status,rating,startedAt,completedAt,notes,tags,createdAt,updatedAt"]
        for item in items {
            let fields = [
                item.id.uuidString,
                Self.csvEscape(item.title),
                item.category.rawValue,
                item.status.rawValue,
                item.rating.map(String.init) ?? "",
                item.startedAt.map { formatter.string(from: $0) } ?? "",
                item.completedAt.map { formatter.string(from: $0) } ?? "",
                Self.csvEscape(item.notes),
                Self.csvEscape(item.tags.joined(separator: ";")),
                formatter.string(from: item.createdAt),
                formatter.string(from: item.updatedAt)
            ]
            lines.append(fields.joined(separator: ","))
        }
        let dateStamp = formatter.string(from: Date())
        let url = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Downloads/luma-media-\(dateStamp).csv")
        try lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    private func refreshCache() async {
        cachedItems = await store.all()
    }

    private func emptyQueryResult() async -> ModuleResult {
        var items: [ResultItem] = [manageLogResult()]
        let recent = MediaIndex.recent(cachedItems, limit: 8)
        items.append(contentsOf: recent.map { itemResult($0) })
        if items.count == 1 {
            let count = cachedItems.count
            let subtitle = count == 0 ? "No items logged yet" : "\(count) items · type `m log` to manage"
            let payload = (try? ModuleActionCoding.encode(MediaAction.openDetail)) ?? Data()
            return ModuleResult(items: [
                ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "empty"),
                    title: "Media Log",
                    titleAttributed: AttributedString("Media Log"),
                    subtitle: subtitle,
                    icon: .symbol("film"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                        title: "Open Media Log",
                        kind: .custom(payload: payload, handler: Self.manifest.identifier)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority)
                )
            ])
        }
        return ModuleResult(items: items)
    }

    private func manageLogResult() -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(MediaAction.openDetail)) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "manage"),
            title: "Media Log",
            titleAttributed: AttributedString("Media Log"),
            subtitle: "Open full log view",
            icon: .symbol("film.stack"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Media Log",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 1)
        )
    }

    private func captureResult(_ parsed: MediaParser.Result) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "capture.\(parsed.title)")
        var subtitleParts: [String] = []
        if let category = parsed.category { subtitleParts.append(category.displayName.lowercased()) }
        if let rating = parsed.rating { subtitleParts.append("★\(rating)") }
        subtitleParts.append(parsed.status.displayName)
        if case .capture(true) = parsed.mode { subtitleParts.append("needs category") }
        let subtitle = subtitleParts.joined(separator: " · ")

        let draft = MediaEditorDraft(
            title: parsed.title,
            category: parsed.category,
            status: parsed.status,
            rating: parsed.rating,
            completedAt: parsed.status == .done ? Date() : nil
        )

        let primary: Action
        if case .capture(false) = parsed.mode, parsed.category != nil {
            primary = Action(
                id: ActionID(module: Self.manifest.identifier, key: "capture"),
                title: "Log Item",
                kind: .custom(
                    payload: (try? ModuleActionCoding.encode(MediaAction.capture(draft))) ?? Data(),
                    handler: Self.manifest.identifier
                )
            )
        } else {
            primary = Action(
                id: ActionID(module: Self.manifest.identifier, key: "edit-draft"),
                title: "Complete Entry",
                kind: .custom(
                    payload: (try? ModuleActionCoding.encode(MediaAction.editDraft(draft))) ?? Data(),
                    handler: Self.manifest.identifier
                )
            )
        }

        return ResultItem(
            id: id,
            title: "Log \(parsed.title)",
            titleAttributed: AttributedString("Log \(parsed.title)"),
            subtitle: subtitle,
            icon: .symbol(parsed.category?.symbolName ?? "plus.circle"),
            primaryAction: primary,
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 2)
        )
    }

    private func itemResult(_ item: MediaItem) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: item.id.uuidString)
        var subtitleParts = [item.category.displayName, item.status.verb(for: item.category)]
        if let rating = item.rating { subtitleParts.append("★\(rating)") }
        return ResultItem(
            id: id,
            title: item.title,
            titleAttributed: AttributedString(item.title),
            subtitle: subtitleParts.joined(separator: " · "),
            icon: .symbol(item.category.symbolName),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "edit.\(item.id.uuidString)"),
                title: "Edit Item",
                kind: .custom(
                    payload: (try? ModuleActionCoding.encode(MediaAction.edit(id: item.id))) ?? Data(),
                    handler: Self.manifest.identifier
                )
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "copy.\(item.id.uuidString)"),
                    title: "Copy Summary",
                    kind: .custom(
                        payload: (try? ModuleActionCoding.encode(MediaAction.copy(id: item.id))) ?? Data(),
                        handler: Self.manifest.identifier
                    )
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private static func summaryLine(for item: MediaItem) -> String {
        if let rating = item.rating {
            return "\(item.title) — \(rating)/10"
        }
        return item.title
    }

    private static func csvEscape(_ value: String) -> String {
        let escaped = value.replacingOccurrences(of: "\"", with: "\"\"")
        return "\"\(escaped)\""
    }
}
