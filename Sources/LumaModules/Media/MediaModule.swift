import Foundation
import LumaCore

public actor MediaModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .media,
        displayName: "Records",
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
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
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
            let searchQuery = parsed.title.isEmpty && !parsed.tags.isEmpty
                ? parsed.tags.map { "#\($0)" }.joined(separator: " ")
                : parsed.title
            let matches = MediaIndex.search(cachedItems, query: searchQuery, limit: 8)
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
                tags: parsed.tags,
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
        case .openDetail, .edit, .editDraft:
            break
        case .capture(let draft):
            _ = try await store.add(from: draft)
            await refreshCache()
            await context.launcherUI.reloadModuleDetail(.media)
        case .copy(let id):
            guard let item = cachedItems.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            let summary = Self.summaryLine(for: item)
            await context.platform.pasteboard.write(summary)
        }
    }

    public func allItems() async -> [MediaItem] {
        await store.all()
    }

    public func inProgressCount() async -> Int {
        if cachedItems.isEmpty {
            cachedItems = await store.all()
        }
        return cachedItems.filter { $0.status == .inProgress }.count
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
            .appendingPathComponent("Downloads/luma-records-\(dateStamp).csv")
        try lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// Returns the text after a Records trigger prefix, or nil if the query does not target Records.
    static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "m" || lower == "media" || lower == "rec" || lower == "record" || lower == "log" {
            return ""
        }
        if lower.hasPrefix("m ") {
            return String(trimmed.dropFirst(2)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("media ") {
            return String(trimmed.dropFirst(6)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("rec ") {
            return String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("record ") {
            return String(trimmed.dropFirst(7)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("log ") {
            return String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    private func refreshCache() async {
        cachedItems = await store.all()
    }

    private func emptyQueryResult() async -> ModuleResult {
        let recent = MediaIndex.recent(cachedItems, limit: 8)
        guard !recent.isEmpty else {
            return ModuleResult(items: [])
        }
        return ModuleResult(items: recent.map { itemResult($0) })
    }

    private func manageLogResult() -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(MediaAction.openDetail)) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "manage"),
            title: "Records",
            titleAttributed: AttributedString("Records"),
            subtitle: "Open full logbook",
            icon: .symbol("books.vertical"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Records",
                kind: .openModuleDetail(Self.manifest.identifier, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 1),
            rowKind: .starter
        )
    }

    private func captureResult(_ parsed: MediaParser.Result) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "capture.\(parsed.title)")
        var subtitleParts: [String] = []
        if let category = parsed.category {
            subtitleParts.append(category.displayName)
            subtitleParts.append(parsed.status.verb(for: category))
        } else if parsed.status != .done {
            subtitleParts.append(parsed.status.displayName)
        }
        if let rating = parsed.rating { subtitleParts.append("★\(rating)") }
        subtitleParts.append(contentsOf: parsed.tags.map { "#\($0)" })
        if case .capture(true) = parsed.mode { subtitleParts.append("needs category") }
        let subtitle = subtitleParts.joined(separator: " · ")

        let draft = MediaEditorDraft(
            title: parsed.title,
            category: parsed.category,
            status: parsed.status,
            rating: parsed.rating,
            completedAt: parsed.status == .done ? Date() : nil,
            tags: parsed.tags
        )

        let isComplete = parsed.category != nil
        let primary: Action
        if case .capture(false) = parsed.mode, isComplete {
            primary = Action(
                id: ActionID(module: Self.manifest.identifier, key: "capture"),
                title: "Log Item",
                kind: .custom(
                    payload: (try? ModuleActionCoding.encode(MediaAction.capture(draft))) ?? Data(),
                    handler: Self.manifest.identifier
                )
            )
        } else {
            let draftPayload = (try? ModuleActionCoding.encode(MediaAction.editDraft(draft))) ?? Data()
            primary = Action(
                id: ActionID(module: Self.manifest.identifier, key: "edit-draft"),
                title: "Complete Entry",
                kind: .openModuleDetail(Self.manifest.identifier, payload: draftPayload)
            )
        }

        let title = isComplete ? "Log \(parsed.title)" : "Complete Entry"
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
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
        subtitleParts.append(contentsOf: item.tags.map { "#\($0)" })
        let editPayload = (try? ModuleActionCoding.encode(MediaAction.editDraft(MediaEditorDraft(item: item)))) ?? Data()
        return ResultItem(
            id: id,
            title: item.title,
            titleAttributed: AttributedString(item.title),
            subtitle: subtitleParts.joined(separator: " · "),
            icon: .symbol(item.category.symbolName),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "edit.\(item.id.uuidString)"),
                title: "Edit Item",
                kind: .openModuleDetail(Self.manifest.identifier, payload: editPayload)
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
