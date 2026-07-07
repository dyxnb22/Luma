import Foundation
import LumaCore

public enum NotesDailyCaptureOutcome: Sendable {
    case appended(URL)
    case rootNotConfigured
    case failed
}

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
    private let metaIndex: NotesMetaIndex
    private let configStore: NotesRootConfigStore
    private var fileSystem: any FileSystemClient = NoopFileSystemClient()
    private var watchTask: Task<Void, Never>?
    private var watchRoot: URL?
    private var rootPath: String?
    private var cachedConfig: NotesRootConfig = .empty
    private var templates: [NotesTemplateInfo] = []
    private var lastWarmupDurationMs: Double = 0
    private static let recentNotesLimit = 8

    public init() {
        index = NotesTreeIndex()
        metaIndex = NotesMetaIndex()
        configStore = NotesRootConfigStore()
    }

    public init(index: NotesTreeIndex, config: NotesRootConfigStore, metaIndex: NotesMetaIndex = NotesMetaIndex()) {
        self.index = index
        self.metaIndex = metaIndex
        configStore = config
    }

    public func warmup(_ context: ModuleContext) async {
        fileSystem = context.platform.fileSystem
        await reloadFromConfig()
    }

    public func teardown() async {
        watchTask?.cancel()
        watchTask = nil
        if let watchRoot {
            await fileSystem.stopWatching(root: watchRoot)
        }
        self.watchRoot = nil
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? NotesQueryParser.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        let parsed = NotesQueryParser.parse(
            payload: payload,
            knownTemplates: NotesTemplateStore.templateNames(from: templates)
        )

        switch parsed {
        case .help:
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        case .listRecents:
            guard cachedConfig.root != nil else {
                return ModuleResult(items: [noRootRow()])
            }
            let recents = await recentNotePaths()
            let items = recents.prefix(8).map { path in
                let name = URL(fileURLWithPath: path).deletingPathExtension().lastPathComponent
                let node = NotesNode(path: path, name: name, kind: .note, children: [])
                return result(for: node)
            }
            return ModuleResult(items: Array(items))
        case .search(let text):
            let matches = await index.search(fuzzy: text, limit: 10)
            return ModuleResult(items: matches.map(result(for:)))
        case .metaSearch(let filter):
            let matches = await metaIndex.search(filter: filter, limit: 12)
            return ModuleResult(items: matches.map { result(for: metaToNode($0)) })
        case .new(let title, let template):
            return await captureResult(title: title, template: template)
        case .daily:
            return await dailyResult()
        case .reviewWeek:
            return await reviewWeekResult()
        case .doctor:
            return await doctorResult()
        case .captureToDaily(let text):
            return await captureToDailyResult(text: text)
        }
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind else {
            throw ModuleError.unsupportedAction(action.id)
        }
        guard handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(NotesAction.self, from: payload)
        let config = cachedConfig
        guard let root = config.root else { throw NotesActionError.rootMissing }

        let actions = NotesActions(index: index)
        let url: URL
        switch decoded {
        case .open(let path):
            do {
                try PathContainment.validateContained(path: path, in: root)
            } catch PathContainmentError.pathOutsideRoot {
                throw NotesActionError.pathOutsideRoot
            }
            url = URL(fileURLWithPath: path)
            await recordRecent(path: path)
            try await context.platform.workspace.openLocalFileURL(url)
            return
        case .createInInbox(let title):
            url = try await actions.createNoteInInbox(
                title: title,
                root: root,
                inboxFolderName: config.inboxFolderName
            )
        case .createFromTemplate(let templateName, let title):
            guard let template = NotesTemplateStore.template(named: templateName, in: templates) else {
                throw NotesActionError.templateNotFound
            }
            url = try await actions.createNoteFromTemplate(
                template: template,
                title: title,
                root: root,
                inboxFolderName: config.inboxFolderName
            )
        case .openOrCreateDaily:
            url = try await actions.openOrCreateDailyNote(
                root: root,
                dailyFolderName: config.dailyFolderName
            )
        case .createWeeklyReview:
            let weekStart = Calendar.current.date(byAdding: .day, value: -7, to: Date()) ?? Date()
            let modified = await metaIndex.modifiedSince(weekStart)
            url = try await actions.createWeeklyReview(
                root: root,
                reviewsFolderName: config.reviewsFolderName,
                modifiedNotes: modified
            )
        case .captureToDaily(let text):
            url = try await actions.appendToDailyNote(
                text: text,
                root: root,
                dailyFolderName: config.dailyFolderName
            )
        }

        await recordRecent(path: url.path)
        try await context.platform.workspace.openLocalFileURL(url)
    }

    public func recordOpenedNote(path: String) async {
        await recordRecent(path: path)
    }

    private func recordRecent(path: String) async {
        var config = cachedConfig
        config.recent.removeAll { $0 == path }
        config.recent.insert(path, at: 0)
        if config.recent.count > Self.recentNotesLimit {
            config.recent = Array(config.recent.prefix(Self.recentNotesLimit))
        }
        cachedConfig = config
        try? await configStore.save(config)
    }

    public func recentNotePaths() async -> [String] {
        cachedConfig.recent
    }

    public func loadConfig() async -> NotesRootConfig {
        cachedConfig
    }

    public func saveConfig(_ config: NotesRootConfig) async throws {
        cachedConfig = config
        try await configStore.save(config)
    }

    public func snapshot() async -> NotesNode? {
        await index.snapshot()
    }

    public func notesMetaIndex() -> NotesMetaIndex {
        metaIndex
    }

    public func lastWarmupMilliseconds() async -> Double {
        lastWarmupDurationMs
    }

    public func inboxCount() async -> Int {
        let config = cachedConfig
        guard let root = config.root else { return 0 }
        let actions = NotesActions(index: index)
        return await actions.notesInFolder(named: config.inboxFolderName, root: root).count
    }

    public func dailyNotePath() async -> String? {
        let config = cachedConfig
        guard let root = config.root else { return nil }
        let fileName = NotesActions.dailyFileName(for: Date()) + ".md"
        let path = root.appendingPathComponent(config.dailyFolderName).appendingPathComponent(fileName).path
        return await noteExistsInIndex(path: path) ? path : nil
    }

    public func captureTextToDailyNote(_ text: String) async -> NotesDailyCaptureOutcome {
        let config = cachedConfig
        guard let root = config.root else { return .rootNotConfigured }
        let actions = NotesActions(index: index)
        do {
            let url = try await actions.appendToDailyNote(
                text: text,
                root: root,
                dailyFolderName: config.dailyFolderName
            )
            await recordRecent(path: url.path)
            return .appended(url)
        } catch {
            return .failed
        }
    }

    public func reloadFromConfig() async {
        watchTask?.cancel()
        watchTask = nil
        if let watchRoot {
            await fileSystem.stopWatching(root: watchRoot)
        }
        self.watchRoot = nil

        let config = await configStore.load()
        cachedConfig = config
        rootPath = config.root?.path
        guard let root = config.root else {
            templates = []
            await index.setRoot(nil)
            await metaIndex.rebuild(from: nil)
            return
        }

        templates = NotesTemplateStore.scanTemplates(root: root, folderName: config.templatesFolderName)
        await index.setRoot(root)
        let warmupStart = ContinuousClock.now
        await index.warmup()
        await metaIndex.rebuild(from: await index.snapshot())
        let warmupComponents = warmupStart.duration(to: .now).components
        lastWarmupDurationMs = Double(warmupComponents.seconds) * 1000
            + Double(warmupComponents.attoseconds) / 1_000_000_000_000_000
        await startWatching(root: root)
    }

    public func treeIndex() -> NotesTreeIndex {
        index
    }

    public func detailContentRevision() async -> UInt64 {
        await index.contentRevision()
    }

    private func startWatching(root: URL) async {
        watchRoot = root
        let stream = await fileSystem.watch(root: root, debounceMillis: 200)
        watchTask = Task { [index, metaIndex] in
            for await batch in stream {
                if Task.isCancelled { break }
                await index.rebuild(after: batch)
                let snapshot = await index.snapshot()
                for event in batch {
                    if event.kind == .overflow {
                        CrashLogRecording.record("fsevents.overflow root=\(watchRoot?.path ?? "")")
                    }
                    switch event.kind {
                    case .removed:
                        await metaIndex.remove(path: event.path)
                    default:
                        await metaIndex.update(path: event.path, in: snapshot)
                    }
                }
            }
        }
    }

    private func reviewWeekResult() async -> ModuleResult {
        let config = cachedConfig
        guard config.root != nil else {
            return ModuleResult(items: [noRootRow()])
        }

        let weekLabel = NotesActions.weeklyReviewFileName(for: Date()).replacingOccurrences(of: ".md", with: "")
        let id = ResultID(module: Self.manifest.identifier, key: "review.week")
        let payload = (try? ModuleActionCoding.encode(NotesAction.createWeeklyReview)) ?? Data()
        return ModuleResult(items: [
            ResultItem(
                id: id,
                title: "Create weekly review",
                titleAttributed: AttributedString("Create weekly review"),
                subtitle: "Reviews/\(weekLabel) · prefill modified notes",
                icon: .symbol("calendar.badge.clock"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "review.week"),
                    title: "Create Review",
                    kind: .custom(payload: payload, handler: Self.manifest.identifier)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        ])
    }

    private func metaToNode(_ meta: NotesMeta) -> NotesNode {
        NotesNode(path: meta.path, name: meta.name, kind: .note, children: [])
    }

    private func doctorResult() async -> ModuleResult {
        let config = cachedConfig
        guard config.root != nil else {
            return ModuleResult(items: [noRootRow()])
        }

        let tree = await index.snapshot()
        let (issues, stats) = await NotesDoctor.diagnose(
            tree: tree,
            lastWarmupMilliseconds: lastWarmupDurationMs
        )

        var items = [doctorStatsRow(stats)]
        if issues.isEmpty {
            items.append(doctorHealthyRow())
        } else {
            items.append(contentsOf: issues.map(doctorIssueRow))
        }
        return ModuleResult(items: items)
    }

    private func doctorStatsRow(_ stats: NotesHealthStats) -> ResultItem {
        let title = "Vault: \(stats.noteCount) notes · warmup \(Int(stats.lastWarmupMilliseconds))ms"
        let id = ResultID(module: Self.manifest.identifier, key: "doctor.stats")
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: "n doctor",
            icon: .symbol("stethoscope"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "doctor.stats"),
                title: "Stats",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .informational
        )
    }

    private func doctorHealthyRow() -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "doctor.ok")
        return ResultItem(
            id: id,
            title: "No issues found",
            titleAttributed: AttributedString("No issues found"),
            subtitle: "Frontmatter, links, and names look good",
            icon: .symbol("checkmark.seal"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "doctor.ok"),
                title: "OK",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .informational
        )
    }

    private func doctorIssueRow(_ issue: NotesHealthIssue) -> ResultItem {
        let fileName = URL(fileURLWithPath: issue.path).lastPathComponent
        let id = ResultID(module: Self.manifest.identifier, key: "doctor.\(issue.path)")
        let payload = (try? ModuleActionCoding.encode(NotesAction.open(path: issue.path))) ?? Data()
        return ResultItem(
            id: id,
            title: fileName,
            titleAttributed: AttributedString(fileName),
            subtitle: issue.message,
            icon: .symbol(issue.kind == .brokenLink ? "link.badge.plus" : "exclamationmark.triangle"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(issue.path)"),
                title: "Open in Typora",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func captureResult(title: String, template: String?) async -> ModuleResult {
        let config = cachedConfig
        guard config.root != nil else {
            return ModuleResult(items: [noRootRow()])
        }

        let subtitle: String
        let action: NotesAction
        if let template {
            subtitle = "New from \(template) · Inbox · Return to create and open"
            action = .createFromTemplate(template: template, title: title)
        } else {
            subtitle = "New in Inbox · Return to create and open"
            action = .createInInbox(title: title)
        }

        let id = ResultID(module: Self.manifest.identifier, key: "capture.\(title)")
        let payload = (try? ModuleActionCoding.encode(action)) ?? Data()
        return ModuleResult(items: [
            ResultItem(
                id: id,
                title: title,
                titleAttributed: AttributedString(title),
                subtitle: subtitle,
                icon: .symbol("square.and.pencil"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "capture"),
                    title: "Create Note",
                    kind: .custom(payload: payload, handler: Self.manifest.identifier)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        ])
    }

    private func dailyResult() async -> ModuleResult {
        let config = cachedConfig
        guard let root = config.root else {
            return ModuleResult(items: [noRootRow()])
        }

        let fileName = NotesActions.dailyFileName(for: Date()) + ".md"
        let relative = "\(config.dailyFolderName)/\(fileName)"
        let absolute = root.appendingPathComponent(relative).path
        let exists = await noteExistsInIndex(path: absolute)
        let title = exists ? "Open today's daily note" : "Create today's daily note"
        let subtitle = relative

        let id = ResultID(module: Self.manifest.identifier, key: "daily")
        let payload = (try? ModuleActionCoding.encode(NotesAction.openOrCreateDaily)) ?? Data()
        return ModuleResult(items: [
            ResultItem(
                id: id,
                title: title,
                titleAttributed: AttributedString(title),
                subtitle: subtitle,
                icon: .symbol("calendar"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "daily"),
                    title: exists ? "Open Daily Note" : "Create Daily Note",
                    kind: .custom(payload: payload, handler: Self.manifest.identifier)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        ])
    }

    private func captureToDailyResult(text: String) async -> ModuleResult {
        let config = cachedConfig
        guard config.root != nil else {
            return ModuleResult(items: [noRootRow()])
        }
        let preview = String(text.prefix(64))
        let payload = (try? ModuleActionCoding.encode(NotesAction.captureToDaily(text: text))) ?? Data()
        return ModuleResult(items: [
            ResultItem(
                id: ResultID(module: Self.manifest.identifier, key: "capture.daily"),
                title: "Append to today's daily note",
                titleAttributed: AttributedString("Append to today's daily note"),
                subtitle: preview,
                icon: .symbol("square.and.pencil"),
                primaryAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "capture.daily"),
                    title: "Append to Note",
                    kind: .custom(payload: payload, handler: Self.manifest.identifier)
                ),
                rankingHints: RankingHints(basePriority: Self.manifest.priority)
            )
        ])
    }

    private func noteExistsInIndex(path: String) async -> Bool {
        guard let tree = await index.snapshot() else { return false }
        return flattenNotes(tree).contains { $0.path == path }
    }

    private func flattenNotes(_ node: NotesNode) -> [NotesNode] {
        var results: [NotesNode] = []
        if node.kind == .note { results.append(node) }
        for child in node.children {
            results.append(contentsOf: flattenNotes(child))
        }
        return results
    }

    private func noRootRow() -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "no-root")
        return ResultItem(
            id: id,
            title: "Choose a Notes root folder",
            titleAttributed: AttributedString("Choose a Notes root folder"),
            subtitle: "Open Notes detail or Settings → Notes",
            icon: .symbol("folder.badge.questionmark"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Notes",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
        )
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
