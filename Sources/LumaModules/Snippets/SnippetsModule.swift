import Foundation
import LumaCore
import LumaServices

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
        let normalized = query.normalized
        guard normalized == "s" || normalized == "snip" || normalized.hasPrefix("s ") || normalized.hasPrefix("snip ") else {
            return ModuleResult(items: [])
        }

        let searchText: String
        if normalized == "s" || normalized == "snip" {
            searchText = ""
        } else if normalized.hasPrefix("s ") {
            searchText = String(query.raw.dropFirst(2)).trimmingCharacters(in: .whitespacesAndNewlines)
        } else {
            searchText = String(query.raw.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
        }

        if ModuleHelp.isHelpQuery(searchText) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

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
            await context.pasteboard.write(SnippetVariableExpander.expand(snippet.content))
        case .paste(let id):
            guard let snippet = cachedSnippets.first(where: { $0.id == id }) else {
                throw ModuleError.dataUnavailable
            }
            _ = try await store.recordUsage(id: id)
            await refreshCache()
            let expanded = SnippetVariableExpander.expand(snippet.content)
            await context.pasteboard.write(expanded)
            if AXService.isProcessTrusted() {
                await context.accessibility.insert(text: snippet.content)
            }
        }
    }

    public func allSnippets() async -> [Snippet] {
        await store.all()
    }

    public func count() async -> Int {
        cachedSnippets.count
    }

    public func add(title: String, content: String, tags: [String]) async throws -> Snippet {
        let snippet = try await store.add(title: title, content: content, tags: tags)
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

    private func refreshCache() async {
        cachedSnippets = await store.all()
    }

    private func emptyLibraryResult() -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "empty")
        return ResultItem(
            id: id,
            title: "Snippets",
            titleAttributed: AttributedString("Snippets"),
            subtitle: "No snippets yet — open the dashboard card to add one",
            icon: .symbol("text.cursor"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "noop"),
                title: "Snippets",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
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
}
