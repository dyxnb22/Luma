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
        await refreshIndex()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let lower = payload.lowercased()
        if lower == "manage" {
            return manageResult()
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
            try await ProjectOpenerRunner.open(path: path, opener: opener, workspace: context.platform.workspace)
            try await store.recordOpened(path: path)
            await refreshIndex()
        case .copyPath(let path):
            await context.platform.pasteboard.write(path)
        case .reveal(let path):
            await context.platform.workspace.revealInFinder(URL(fileURLWithPath: path, isDirectory: true))
        case .revealConfig:
            let url = await store.configFileURL()
            await context.platform.workspace.revealInFinder(url)
        case .openCurrentDetail:
            break
        case .openManage:
            break
        case .openTerminal(let path):
            try await ProjectOpenerRunner.open(path: path, opener: .terminal, workspace: context.platform.workspace)
            try await store.recordOpened(path: path)
            await refreshIndex()
        case .openNotes(let path, let projectName):
            let candidates = ProjectNotesPaths.candidates(projectPath: path, projectName: projectName)
            if let existing = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) {
                await context.platform.workspace.openURL(existing)
            } else {
                await context.platform.workspace.revealInFinder(URL(fileURLWithPath: path, isDirectory: true))
            }
        case .togglePin(let path):
            try await store.togglePin(path: path)
            await refreshIndex()
        case .updateAliases(let path, let aliases):
            var config = await store.current()
            let normalized = ProjectRecord.normalizePath(path)
            if let idx = config.projects.firstIndex(where: { $0.path == normalized }) {
                config.projects[idx].aliases = aliases
            } else if let scanned = scannedRecords.first(where: { $0.path == normalized }) {
                var record = scanned
                record.aliases = aliases
                config.projects.append(record)
            }
            try await store.save(config)
            await refreshIndex()
        case .updateOpener(let path, let opener):
            var config = await store.current()
            let normalized = ProjectRecord.normalizePath(path)
            if let idx = config.projects.firstIndex(where: { $0.path == normalized }) {
                config.projects[idx].preferredOpener = opener
            } else if let scanned = scannedRecords.first(where: { $0.path == normalized }) {
                var record = scanned
                record.preferredOpener = opener
                config.projects.append(record)
            }
            try await store.save(config)
            await refreshIndex()
        case .addRoot(let path):
            try await store.addRoot(path)
            scannedRecords = ProjectScanner.scan(roots: (await store.current()).roots)
            await refreshIndex()
        case .addManualProject(let name, let path):
            let record = ProjectRecord(name: name, path: path)
            try await store.upsertProject(record)
            await refreshIndex()
        }
    }

    public func matchByLabel(_ label: String) -> ProjectRecord? {
        index.matchByLabel(label)
    }

    public func allRecords() async -> [ProjectRecord] {
        await store.allRecordsIncludingScanned(scannedRecords)
    }

    public func roots() async -> [String] {
        await store.current().roots
    }

    public func isManualProject(path: String) async -> Bool {
        await store.isManualProject(path: path)
    }

    public func configFileURL() async -> URL {
        await store.configFileURL()
    }

    private func refreshIndex() async {
        let config = await store.current()
        scannedRecords = ProjectScanner.scan(roots: config.roots)
        index = ProjectIndex(records: config.projects + scannedRecords)
    }

    private func manageResult() -> ModuleResult {
        let payload = (try? ModuleActionCoding.encode(ProjectAction.openManage)) ?? Data()
        return ModuleResult(items: [
            ResultItem(
                id: ResultID(module: Self.manifest.identifier, key: "manage"),
                title: "Manage Projects",
                titleAttributed: AttributedString("Manage Projects"),
                subtitle: "Pin, aliases, roots",
                icon: .symbol("folder.badge.gearshape"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "manage"),
                    title: "Manage Projects",
                    kind: .openModuleDetail(Self.manifest.identifier, payload: payload)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        ])
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

        let notesPayload = (try? ModuleActionCoding.encode(ProjectAction.openNotes(path: record.path, projectName: record.name))) ?? Data()
        secondary.append(Action(
            id: ActionID(module: Self.manifest.identifier, key: "notes.\(key)"),
            title: CrossModuleActionTitles.openNotesForProject,
            kind: .custom(payload: notesPayload, handler: Self.manifest.identifier)
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
