import AppKit
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
    private var pollingTask: Task<Void, Never>?
    private var lastChangeCount = -1

    public init(fileManager: FileManager = .default) {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        let persistenceURL = base.appendingPathComponent("Luma/clipboard-history.json")
        try? fileManager.createDirectory(at: persistenceURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        self.persistenceURL = persistenceURL
        store = ClipboardHistoryStore(persistenceURL: persistenceURL)
    }

    public func warmup(_ context: ModuleContext) async {
        let maxEntries = await context.config.clipboardMaxEntries()
        let maxAgeDays = await context.config.clipboardMaxAgeDays()
        let maxEntrySizeKB = await context.config.clipboardMaxEntrySizeKB()
        store = ClipboardHistoryStore(
            maxEntries: maxEntries,
            maxAge: TimeInterval(maxAgeDays * 24 * 60 * 60),
            maxTextBytes: maxEntrySizeKB * 1024,
            persistenceURL: persistenceURL
        )
        startPolling()
    }

    public func teardown() async {
        pollingTask?.cancel()
        pollingTask = nil
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized
        if normalized == "clip ?" || normalized == "clip help" {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        let entries = await store.search(query.normalized, limit: 10)
        return ModuleResult(items: entries.map(result))
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
        let current = await store.list(filter: .all, query: "", limit: 500)
        guard let entry = current.first(where: { $0.id == id }) else { return }
        await store.pin(id, isPinned: !entry.isPinned)
    }

    public func remove(_ id: UUID) async {
        await store.removeEntry(id)
    }

    public func clearUnpinned() async {
        await store.clearUnpinned()
    }

    public func applyRetentionSettings(maxEntries: Int, maxAgeDays: Int, maxEntrySizeKB: Int) async {
        await store.updateRetention(
            maxEntries: maxEntries,
            maxAge: TimeInterval(maxAgeDays * 24 * 60 * 60),
            maxTextBytes: maxEntrySizeKB * 1024
        )
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

    private func capturePasteboardIfNeeded() async {
        let snapshot = await MainActor.run { () -> (Int, [String], String?, String?, String?) in
            let pasteboard = NSPasteboard.general
            let changeCount = pasteboard.changeCount
            let types = pasteboard.types?.map(\.rawValue) ?? []
            let text = pasteboard.string(forType: .string)
            let frontmost = NSWorkspace.shared.frontmostApplication
            let bundleID = frontmost?.bundleIdentifier
            let appName = frontmost?.localizedName
            let lumaBundleID = Bundle.main.bundleIdentifier
            if bundleID == lumaBundleID {
                return (changeCount, types, text, nil, nil)
            }
            return (changeCount, types, text, appName, bundleID)
        }

        guard snapshot.0 != lastChangeCount else { return }
        lastChangeCount = snapshot.0
        let types = snapshot.1
        if let text = snapshot.2, !text.isEmpty {
            await store.add(text: text, types: types, sourceAppName: snapshot.3, sourceBundleID: snapshot.4)
            return
        }
        if ClipboardEntryKind.isImageTypes(types) {
            let imagePayload = await MainActor.run { () -> (Data?, String?) in
                let pasteboard = NSPasteboard.general
                for type in ["public.png", "public.tiff", "public.jpeg", "public.image"] {
                    if let data = pasteboard.data(forType: NSPasteboard.PasteboardType(type)) {
                        return (data, type)
                    }
                }
                if let data = pasteboard.data(forType: .tiff) {
                    return (data, NSPasteboard.PasteboardType.tiff.rawValue)
                }
                return (nil, nil)
            }
            guard let data = imagePayload.0 else { return }
            await store.add(
                text: "[Image]",
                types: types,
                sourceAppName: snapshot.3,
                sourceBundleID: snapshot.4,
                imageData: data,
                imagePasteboardType: imagePayload.1
            )
        }
    }

    private func result(for entry: ClipboardEntry) -> ResultItem {
        let title = entry.text.count > 80 ? String(entry.text.prefix(80)) + "..." : entry.text
        let id = ResultID(module: Self.manifest.identifier, key: entry.id.uuidString)
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: entry.isPinned ? "Pinned Clipboard" : "Clipboard",
            icon: .symbol("doc.on.clipboard"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(entry.id.uuidString)"),
                title: "Copy",
                kind: .copyToPasteboard(entry.text)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
