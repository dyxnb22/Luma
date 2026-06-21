import Foundation
import LumaCore

public actor NotesModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .notes,
        displayName: "Notes Graph",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(40)
    )

    private var store = NotesVaultStore(vaultURL: FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Documents/LumaNotes"))

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let searchText = query.raw
            .replacingOccurrences(of: "notes", with: "", options: [.caseInsensitive])
            .replacingOccurrences(of: "note", with: "", options: [.caseInsensitive])
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let notes = await store.scan()
        if !searchText.isEmpty {
            let matches = notes
                .filter { $0.title.localizedCaseInsensitiveContains(searchText) || $0.body.localizedCaseInsensitiveContains(searchText) }
                .prefix(10)
                .map(noteResult)
            if !matches.isEmpty || query.normalized.hasPrefix("note") {
                return ModuleResult(items: Array(matches))
            }
        }

        guard query.normalized == "notes" || query.normalized == "note" else { return ModuleResult(items: []) }

        let id = ResultID(module: Self.manifest.identifier, key: "open-vault")
        let item = ResultItem(
            id: id,
            title: "Open Notes Vault",
            titleAttributed: AttributedString("Open Notes Vault"),
            subtitle: "Markdown tree and graph",
            icon: .symbol("note.text"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-vault"),
                title: "Open Notes",
                kind: .custom(payload: Data(), handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
        return ModuleResult(items: [item])
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard action.id.key.hasPrefix("open.") else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let path = String(action.id.key.dropFirst("open.".count))
        let url = URL(fileURLWithPath: path)
        await store.openInTypora(url)
    }

    private func noteResult(_ note: NoteFile) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: note.url.path)
        return ResultItem(
            id: id,
            title: note.title,
            titleAttributed: AttributedString(note.title),
            subtitle: note.url.deletingLastPathComponent().path,
            icon: .symbol("note.text"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(note.url.path)"),
                title: "Open in Typora",
                kind: .custom(payload: Data(), handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
