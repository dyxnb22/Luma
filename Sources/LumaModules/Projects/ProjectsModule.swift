import Foundation
import LumaCore

public actor ProjectsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .projects,
        displayName: "Projects",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 4,
        queryTimeout: .milliseconds(30)
    )

    private let store: ProjectStore
    private var index = ProjectIndex(records: [])
    private var scannedRecords: [ProjectRecord] = []

    public init(store: ProjectStore = ProjectStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        let config = await store.current()
        scannedRecords = ProjectScanner.scan(roots: config.roots)
        index = ProjectIndex(records: config.projects + scannedRecords)
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let config = await store.current()
        let matches: [ProjectRecord]
        if payload.isEmpty {
            matches = index.homeRecords(limit: 8, recentPaths: config.recent)
        } else {
            matches = index.search(payload).map(\.record)
        }

        if matches.isEmpty {
            return ModuleResult(items: [])
        }

        return ModuleResult(items: matches.map { projectRow($0) })
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(ProjectAction.self, from: payload)
        switch decoded {
        case .open(let path, let opener):
            try await ProjectOpenerRunner.open(path: path, opener: opener, workspace: context.workspace)
            try await store.recordOpened(path: path)
            await refreshIndex()
        case .copyPath(let path):
            await context.pasteboard.write(path)
        case .reveal(let path):
            await context.workspace.revealInFinder(URL(fileURLWithPath: path, isDirectory: true))
        case .revealConfig:
            let url = await store.configFileURL()
            await context.workspace.revealInFinder(url)
        }
    }

    private func refreshIndex() async {
        let config = await store.current()
        index = ProjectIndex(records: config.projects + scannedRecords)
    }

    private func projectRow(_ record: ProjectRecord) -> ResultItem {
        let key = record.path
        let subtitle = displayPath(record.path)
        let openPayload = (try? ModuleActionCoding.encode(ProjectAction.open(path: record.path, opener: record.preferredOpener))) ?? Data()

        var secondary: [Action] = []
        for opener in ProjectOpener.allCases where opener != record.preferredOpener {
            let title = secondaryTitle(for: opener)
            let payload = (try? ModuleActionCoding.encode(ProjectAction.open(path: record.path, opener: opener))) ?? Data()
            secondary.append(Action(
                id: ActionID(module: Self.manifest.identifier, key: "\(opener.rawValue).\(key)"),
                title: title,
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ))
        }

        let copyPayload = (try? ModuleActionCoding.encode(ProjectAction.copyPath(record.path))) ?? Data()
        secondary.append(Action(
            id: ActionID(module: Self.manifest.identifier, key: "copy.\(key)"),
            title: "Copy Path",
            kind: .custom(payload: copyPayload, handler: Self.manifest.identifier)
        ))

        let revealConfigPayload = (try? ModuleActionCoding.encode(ProjectAction.revealConfig)) ?? Data()
        secondary.append(Action(
            id: ActionID(module: Self.manifest.identifier, key: "config"),
            title: "Reveal Config",
            kind: .custom(payload: revealConfigPayload, handler: Self.manifest.identifier)
        ))

        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: key),
            title: record.name,
            titleAttributed: AttributedString(record.name),
            subtitle: subtitle,
            icon: .symbol(iconName(for: record.preferredOpener)),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(key)"),
                title: primaryTitle(for: record.preferredOpener),
                kind: .custom(payload: openPayload, handler: Self.manifest.identifier)
            ),
            secondaryActions: secondary,
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func primaryTitle(for opener: ProjectOpener) -> String {
        switch opener {
        case .cursor: "Open in Cursor"
        case .vscode: "Open in VS Code"
        case .finder: "Open in Finder"
        case .terminal: "Open in Terminal"
        }
    }

    private func secondaryTitle(for opener: ProjectOpener) -> String {
        primaryTitle(for: opener)
    }

    private func iconName(for opener: ProjectOpener) -> String {
        switch opener {
        case .cursor, .vscode: "chevron.left.forwardslash.chevron.right"
        case .finder: "folder"
        case .terminal: "terminal"
        }
    }

    private func displayPath(_ path: String) -> String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if path.hasPrefix(home) {
            return "~" + path.dropFirst(home.count)
        }
        return path
    }

    static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "proj" || lower == "project" || lower == "p" {
            return ""
        }
        if lower.hasPrefix("proj ") {
            return String(trimmed.dropFirst("proj ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("project ") {
            return String(trimmed.dropFirst("project ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("p ") {
            return String(trimmed.dropFirst("p ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
