import AppKit
import Foundation

public actor BrowserTabsService {
    public static let shared = BrowserTabsService()

    private let runner: AppleScriptRunner
    private let adapters: [any BrowserAdapter]
    private var cached: [TabRecord] = []
    private var lastRefresh: Date?
    private var refreshTask: Task<Void, Never>?
    private var isRefreshing = false
    private let ttl: TimeInterval = 5

    public init(
        runner: AppleScriptRunner = AppleScriptRunner(),
        adapters: [any BrowserAdapter] = [
            SafariAdapter(),
            ChromiumAdapter(bundleID: "com.google.Chrome", applicationName: "Google Chrome"),
            ChromiumAdapter(bundleID: "com.brave.Browser", applicationName: "Brave Browser"),
            ChromiumAdapter(bundleID: "com.microsoft.edgemac", applicationName: "Microsoft Edge"),
            ChromiumAdapter(bundleID: "company.thebrowser.Browser", applicationName: "Arc")
        ]
    ) {
        self.runner = runner
        self.adapters = adapters
    }

    public func refresh() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        defer {
            isRefreshing = false
            refreshTask = nil
        }
        let runningBundleIDs = await MainActor.run {
            Set(NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier))
        }
        let runnable = adapters.filter { runningBundleIDs.contains($0.bundleID) }
        var records: [TabRecord] = []
        await withTaskGroup(of: [TabRecord].self) { group in
            for adapter in runnable {
                let runner = self.runner
                group.addTask {
                    (try? await adapter.fetchTabs(runner: runner)) ?? []
                }
            }
            for await batch in group {
                records.append(contentsOf: batch)
            }
        }
        cached = records
        lastRefresh = Date()
    }

    public func searchableTabs() async -> [TabRecord] {
        if cached.isEmpty || isStale {
            await refresh()
        }
        return cached
    }

    public func cachedTabs() -> [TabRecord] {
        if isStale, !isRefreshing, refreshTask == nil {
            refreshTask = Task { await self.refresh() }
        }
        return cached
    }

    private var isStale: Bool {
        guard let lastRefresh else { return true }
        return Date().timeIntervalSince(lastRefresh) > ttl
    }

    public func activate(_ record: TabRecord) async throws {
        guard let adapter = adapters.first(where: { $0.bundleID == record.bundleID }) else { return }
        try await adapter.activate(record: record, runner: runner)
    }
}
