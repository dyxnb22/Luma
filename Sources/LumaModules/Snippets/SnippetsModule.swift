import Foundation
import LumaCore

public actor SnippetsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .snippets,
        displayName: "Snippets",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(20)
    )

    private let store: SnippetsStore
    private var cachedSnippets: [Snippet] = []

    public init(store: SnippetsStore = SnippetsStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        cachedSnippets = await store.all()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let lower = payload.lowercased()
        if lower.hasPrefix("new ") {
            let title = String(payload.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !title.isEmpty else { return ModuleResult(items: []) }
            return ModuleResult(items: [createRow(title: title)])
        }

        let searchText = payload

        let matches = SnippetIndex.search(cachedSnippets, query: searchText, limit: 8)
        if matches.isEmpty {
            if searchText.isEmpty {
                return ModuleResult(items: [emptyLibraryResult()])
            }
            return ModuleResult(items: [])
        }

        return ModuleResult(items: matches.map { snippetResult($0.snippet) })
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(SnippetsAction.self, from: payload)

        switch decoded {
        case .copy(let id):
            guard let snippet = cachedSnippets.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            _ = try await store.recordUsage(id: id)
            await refreshCache()
            let expanded = await expandedContent(for: snippet.content, context: context)
            await context.platform.pasteboard.write(expanded)
        case .paste(let id):
            guard let snippet = cachedSnippets.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            _ = try await store.recordUsage(id: id)
            await refreshCache()
            let expanded = await expandedContent(for: snippet.content, context: context)
            await context.platform.pasteboard.write(expanded)
            if await context.platform.accessibility.isTrusted() {
                await context.platform.accessibility.insert(text: expanded)
            }
        case .create:
            break
        case .prepareDraft:
            break
        }
    }

    public func allSnippets() async -> [Snippet] {
        await store.all()
    }

    public func count() async -> Int {
        cachedSnippets.count
    }

    public func add(title: String, content: String, tags: [String], trigger: String = "") async throws -> Snippet {
        let snippet = try await store.add(title: title, content: content, tags: tags, trigger: trigger)
        await refreshCache()
        return snippet
    }

    public func update(_ snippet: Snippet) async throws -> Snippet {
        let updated = try await store.update(snippet)
        await refreshCache()
        return updated
    }

    public func delete(id: UUID) async throws {
        try await store.delete(id: id)
        await refreshCache()
    }

    public func duplicate(id: UUID) async throws -> Snippet {
        let copy = try await store.duplicate(id: id)
        await refreshCache()
        return copy
    }

    public func markUsed(id: UUID) async throws {
        _ = try await store.recordUsage(id: id)
        await refreshCache()
    }

    private func refreshCache() async {
        cachedSnippets = await store.all()
    }

    private func expandedContent(for content: String, context: ActionContext) async -> String {
        let project = await context.platform.currentProject.snapshot()
        let selection = await context.platform.selectionSnapshot.snapshot()
        let clipboard = await context.platform.pasteboard.readString()
        return SnippetVariableExpander.expand(
            content,
            context: SnippetExpansionContext.from(
                project: project,
                clipboardText: clipboard,
                selectionText: selection
            )
        )
    }

    private func createRow(title: String) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(SnippetsAction.create(title: title))) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "create.\(title)"),
            title: "Create Snippet",
            titleAttributed: AttributedString("Create Snippet"),
            subtitle: title,
            icon: .symbol("plus.circle"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "create"),
                title: "Create & Open",
                kind: .openModuleDetail(Self.manifest.identifier, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 2)
        )
    }

    private func emptyLibraryResult() -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "empty")
        return ResultItem(
            id: id,
            title: "Snippets",
            titleAttributed: AttributedString("Snippets"),
            subtitle: "No snippets yet — open Snippets to add one",
            icon: .symbol("text.cursor"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Snippets",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .starter
        )
    }

    private func snippetResult(_ snippet: Snippet) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: snippet.id.uuidString)
        let preview = snippet.content.replacingOccurrences(of: "\n", with: " ")
        let subtitle: String
        if preview.count <= 80 {
            subtitle = preview
        } else {
            subtitle = String(preview.prefix(80)) + "…"
        }
        let tagSuffix = snippet.tags.isEmpty ? "" : " · " + snippet.tags.joined(separator: ", ")
        let copyPayload = (try? ModuleActionCoding.encode(SnippetsAction.copy(id: snippet.id))) ?? Data()
        let pastePayload = (try? ModuleActionCoding.encode(SnippetsAction.paste(id: snippet.id))) ?? Data()
        return ResultItem(
            id: id,
            title: snippet.title,
            titleAttributed: AttributedString(snippet.title),
            subtitle: subtitle + tagSuffix,
            icon: .symbol("text.cursor"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(snippet.id.uuidString)"),
                title: "Copy Snippet",
                kind: .custom(payload: copyPayload, handler: Self.manifest.identifier),
                runsOn: .background
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "paste.\(snippet.id.uuidString)"),
                    title: "Paste Snippet",
                    kind: .custom(payload: pastePayload, handler: Self.manifest.identifier),
                    runsOn: .background
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            displayDensity: .expanded
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "s" || lower == "snip" {
            return ""
        }
        if lower.hasPrefix("s ") {
            return String(trimmed.dropFirst(2)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("snip ") {
            return String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
