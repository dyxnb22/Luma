import Foundation
import LumaCore

public actor ClipboardModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .clipboard,
        displayName: "Clipboard",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(30)
    )

    private var store: ClipboardHistoryStore
    private let persistenceURL: URL
    private let pasteboard: any PasteboardClient
    private let accessibility: any AccessibilityClient
    private var clipboardSnapshot: any ClipboardSnapshotClient = NoopClipboardSnapshotClient()
    private var pollingTask: Task<Void, Never>?
    private var lastChangeCount = -1
    private var suppressedChangeCount: Int?
    private var historyEnabled = true
    private var pasteBehavior = ClipboardPasteBehavior.pasteDirectly

    public init(
        fileManager: FileManager = .default,
        pasteboard: any PasteboardClient = NoopPasteboardClient(),
        accessibility: any AccessibilityClient = NoopAccessibilityClient()
    ) {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        let persistenceURL = base.appendingPathComponent("Luma/clipboard-history.json")
        try? fileManager.createDirectory(at: persistenceURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        self.persistenceURL = persistenceURL
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        store = ClipboardHistoryStore(persistenceURL: persistenceURL)
    }

    init(
        store: ClipboardHistoryStore,
        persistenceURL: URL,
        pasteboard: any PasteboardClient = NoopPasteboardClient(),
        accessibility: any AccessibilityClient = NoopAccessibilityClient(),
        clipboardSnapshot: any ClipboardSnapshotClient = NoopClipboardSnapshotClient()
    ) {
        self.store = store
        self.persistenceURL = persistenceURL
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.clipboardSnapshot = clipboardSnapshot
    }

    public func warmup(_ context: ModuleContext) async {
        clipboardSnapshot = context.platform.clipboardSnapshot
        let maxEntries = await context.runtime.config.clipboardMaxEntries()
        let maxAgeDays = await context.runtime.config.clipboardMaxAgeDays()
        let maxEntrySizeKB = await context.runtime.config.clipboardMaxEntrySizeKB()
        historyEnabled = await context.runtime.config.clipboardHistoryEnabled()
        pasteBehavior = ClipboardPasteBehavior(rawValue: await context.runtime.config.clipboardPasteBehavior()) ?? .pasteDirectly
        let ignored = Set(await context.runtime.config.clipboardIgnoredBundleIDs())
        store = ClipboardHistoryStore(
            maxEntries: maxEntries,
            maxAge: TimeInterval(maxAgeDays * 24 * 60 * 60),
            maxTextBytes: maxEntrySizeKB * 1024,
            persistenceURL: persistenceURL
        )
        await store.updateCapturePolicy(ignoredBundleIDs: ignored)
        await store.persistPrunedStateIfNeeded()
        if historyEnabled {
            startPolling()
        }
    }

    public func teardown() async {
        pollingTask?.cancel()
        pollingTask = nil
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let searchText = payload

        let entries = await store.search(searchText, limit: 10)
        if entries.isEmpty {
            return ModuleResult(items: emptyResults(for: searchText))
        }
        return ModuleResult(items: entries.map(result))
    }

    private func emptyResults(for searchText: String) -> [ResultItem] {
        if searchText.isEmpty {
            return [openDetailRow(
                title: "Open Clipboard history",
                subtitle: "Browse and search captured clips in panel"
            )]
        }
        return [openDetailRow(
            title: "No clipboard matches",
            subtitle: "Open Clipboard to browse full history",
            key: "open-detail.no-matches"
        )]
    }

    private func openDetailRow(title: String, subtitle: String, key: String = "open-detail") -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: key),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .symbol("doc.on.clipboard"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: key),
                title: "Open Clipboard",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority + 1),
            rowKind: .starter
        )
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind,
              handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(ClipboardAction.self, from: payload)
        switch decoded {
        case .copyEntry(let id, let plainTextOnly):
            try await copyEntry(id: id, pasteboard: context.platform.pasteboard, plainTextOnly: plainTextOnly)
        case .pasteEntry(let id):
            try await pasteEntry(id: id)
        case .togglePin(let id):
            await togglePin(id)
        }
    }

    public func copyEntry(id: UUID, pasteboard: (any PasteboardClient)? = nil, plainTextOnly: Bool = false) async throws {
        guard let entry = await store.entry(id: id) else {
            throw ModuleError.dataUnavailable
        }
        try await writeEntry(entry, to: pasteboard ?? self.pasteboard, plainTextOnly: plainTextOnly)
    }

    public func pasteEntry(id: UUID) async throws {
        guard let entry = await store.entry(id: id) else {
            throw ModuleError.dataUnavailable
        }
        try await writeEntry(entry, to: pasteboard, plainTextOnly: false)
        guard pasteBehavior == .pasteDirectly else { return }
        guard entry.imageData == nil, entry.fileURLs?.isEmpty != false else { return }
        try? await Task.sleep(for: .milliseconds(80))
        if await accessibility.isTrusted() {
            await accessibility.insert(text: entry.plainTextForCopy)
        }
    }

    public func recentEntries(limit: Int = 50) async -> [ClipboardEntry] {
        await store.list(filter: .all, query: "", limit: limit)
    }

    public func filteredEntries(filter: ClipboardListFilter, query: String = "", limit: Int = 200) async -> [ClipboardEntry] {
        await store.list(filter: filter, query: query, limit: limit)
    }

    public func statistics() async -> ClipboardStatistics {
        await store.statistics()
    }

    public func togglePin(_ id: UUID) async {
        guard let entry = await store.entry(id: id) else { return }
        await store.pin(id, isPinned: !entry.isPinned)
    }

    @discardableResult
    public func updateEntryText(id: UUID, text: String) async -> Bool {
        await store.updateText(id, text: text)
    }

    public func remove(_ id: UUID) async {
        await store.removeEntry(id)
    }

    public func clearUnpinned() async {
        await store.clearUnpinned()
    }

    public func clearRecent(_ window: ClipboardRecentClearWindow) async {
        await store.clearRecent(window: window)
    }

    public func applyRetentionSettings(maxEntries: Int, maxAgeDays: Int, maxEntrySizeKB: Int) async {
        await store.updateRetention(
            maxEntries: maxEntries,
            maxAge: TimeInterval(maxAgeDays * 24 * 60 * 60),
            maxTextBytes: maxEntrySizeKB * 1024
        )
    }

    public func applyCaptureSettings(
        enabled: Bool,
        ignoredBundleIDs: [String],
        pasteBehavior: ClipboardPasteBehavior
    ) async {
        historyEnabled = enabled
        self.pasteBehavior = pasteBehavior
        await store.updateCapturePolicy(ignoredBundleIDs: Set(ignoredBundleIDs))
        if enabled {
            startPolling()
        } else {
            pollingTask?.cancel()
            pollingTask = nil
        }
    }

    private func writeEntry(_ entry: ClipboardEntry, to pasteboard: any PasteboardClient, plainTextOnly: Bool) async throws {
        if let data = entry.imageData, let type = entry.imagePasteboardType, !plainTextOnly {
            await pasteboard.writeImage(data: data, pasteboardType: type)
        } else if let fileURLs = entry.fileURLs?.map({ URL(fileURLWithPath: $0) }), !fileURLs.isEmpty, !plainTextOnly {
            await pasteboard.writeFileURLs(fileURLs)
        } else {
            await pasteboard.write(entry.plainTextForCopy)
        }
        suppressedChangeCount = (await clipboardSnapshot.readSnapshot()).changeCount
    }

    private func startPolling() {
        pollingTask?.cancel()
        pollingTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.capturePasteboardIfNeeded()
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    func capturePasteboardIfNeeded(snapshot: ClipboardSnapshot? = nil) async {
        guard historyEnabled else { return }

        let board: ClipboardSnapshot
        if let snapshot {
            board = snapshot
        } else {
            board = await clipboardSnapshot.readSnapshot()
        }

        guard board.changeCount != lastChangeCount else { return }
        lastChangeCount = board.changeCount
        guard board.changeCount != suppressedChangeCount else { return }

        if !board.fileURLs.isEmpty {
            let paths = board.fileURLs.map(\.path)
            let label = paths.count == 1
                ? URL(fileURLWithPath: paths[0]).lastPathComponent
                : "[\(paths.count) files]"
            await store.add(
                text: label,
                types: board.types,
                sourceAppName: board.sourceAppName,
                sourceBundleID: board.sourceBundleID,
                fileURLs: paths
            )
            return
        }

        let hasTextRepresentation = ClipboardEntryKind.isTextTypes(board.types)
        if let text = board.text, !text.isEmpty, hasTextRepresentation || board.imageData == nil {
            await store.add(
                text: text,
                types: board.types,
                sourceAppName: board.sourceAppName,
                sourceBundleID: board.sourceBundleID
            )
            return
        }

        if let data = board.imageData, ClipboardEntryKind.isImageTypes(board.types) {
            await store.add(
                text: "[Image]",
                types: board.types,
                sourceAppName: board.sourceAppName,
                sourceBundleID: board.sourceBundleID,
                imageData: data,
                imagePasteboardType: board.imageType
            )
        }
    }

    private func result(for entry: ClipboardEntry) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: entry.id.uuidString)
        let copyPayload = (try? ModuleActionCoding.encode(ClipboardAction.copyEntry(id: entry.id))) ?? Data()
        let pastePayload = (try? ModuleActionCoding.encode(ClipboardAction.pasteEntry(id: entry.id))) ?? Data()
        let pinPayload = (try? ModuleActionCoding.encode(ClipboardAction.togglePin(id: entry.id))) ?? Data()
        var secondary: [Action] = [
            Action(
                id: ActionID(module: Self.manifest.identifier, key: "paste.\(entry.id.uuidString)"),
                title: "Paste",
                kind: .custom(payload: pastePayload, handler: Self.manifest.identifier)
            )
        ]
        if entry.imageData == nil, entry.fileURLs?.isEmpty != false {
            let text = entry.plainTextForCopy.trimmingCharacters(in: .whitespacesAndNewlines)
            if !text.isEmpty {
                let notePayload = (try? ModuleActionCoding.encode(NotesAction.captureToDaily(text: text))) ?? Data()
                secondary.append(Action(
                    id: ActionID(module: Self.manifest.identifier, key: "note.\(entry.id.uuidString)"),
                    title: CrossModuleActionTitles.appendToNote,
                    kind: .custom(payload: notePayload, handler: .notes)
                ))
                let draft = SnippetDraft.fromClipboard(text)
                let snippetPayload = (try? ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))) ?? Data()
                secondary.append(Action(
                    id: ActionID(module: Self.manifest.identifier, key: "snippet.\(entry.id.uuidString)"),
                    title: CrossModuleActionTitles.createSnippet,
                    kind: .openModuleDetail(.snippets, payload: snippetPayload)
                ))
                let plainPayload = (try? ModuleActionCoding.encode(ClipboardAction.copyEntry(id: entry.id, plainTextOnly: true))) ?? Data()
                secondary.append(Action(
                    id: ActionID(module: Self.manifest.identifier, key: "plain.\(entry.id.uuidString)"),
                    title: CrossModuleActionTitles.copyAsPlainText,
                    kind: .custom(payload: plainPayload, handler: Self.manifest.identifier)
                ))
            }
        }
        secondary.append(contentsOf: [
            Action(
                id: ActionID(module: Self.manifest.identifier, key: "pin.\(entry.id.uuidString)"),
                title: entry.isPinned ? "Unpin" : "Pin",
                kind: .custom(payload: pinPayload, handler: Self.manifest.identifier)
            ),
            Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Clipboard",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            )
        ])
        return ResultItem(
            id: id,
            title: entry.metadataLine,
            titleAttributed: AttributedString(entry.metadataLine),
            subtitle: entry.launcherPreviewText,
            icon: .symbol(entry.symbolName),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(entry.id.uuidString)"),
                title: "Copy",
                kind: .custom(payload: copyPayload, handler: Self.manifest.identifier)
            ),
            secondaryActions: secondary,
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            displayDensity: .regular
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "clip" || lower == "cb" {
            return ""
        }
        if lower.hasPrefix("clip ") {
            return String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("cb ") {
            return String(trimmed.dropFirst(3)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
