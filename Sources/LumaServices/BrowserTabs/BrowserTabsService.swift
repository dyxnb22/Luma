import AppKit
import Foundation
import LumaCore

public actor BrowserTabsService {
    public static let shared = BrowserTabsService()

    private let runner: AppleScriptRunner
    private let adapters: [any BrowserAdapter]
    private var cached: [TabRecord] = []
    private var lastRefresh: Date?
    private var refreshTask: Task<Void, Never>?
    private var isRefreshing = false
    private let ttl: TimeInterval = 5
    private var lastRefreshDiagnostic: ModuleDiagnostic?

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
        var issues: [String] = []
        lastRefreshDiagnostic = nil
        await withTaskGroup(of: (String, Result<[TabRecord], Error>).self) { group in
            for adapter in runnable {
                let runner = self.runner
                let browserName = adapter.applicationName
                group.addTask {
                    do {
                        let tabs = try await adapter.fetchTabs(runner: runner)
                        return (browserName, .success(tabs))
                    } catch {
                        return (browserName, .failure(error))
                    }
                }
            }
            for await (browserName, result) in group {
                switch result {
                case .success(let tabs):
                    records.append(contentsOf: tabs)
                case .failure(let error):
                    issues.append(Self.issueMessage(browser: browserName, error: error))
                }
            }
        }
        cached = records
        lastRefresh = Date()
        lastRefreshDiagnostic = Self.diagnostic(records: records, issues: issues)
    }

    public func lastDiagnostic() -> ModuleDiagnostic? {
        lastRefreshDiagnostic
    }

    private static func issueMessage(browser: String, error: Error) -> String {
        switch error {
        case AppleScriptRunner.RunnerError.timedOut:
            return "\(browser) timed out"
        case AppleScriptRunner.RunnerError.failed(let message):
            if message.localizedCaseInsensitiveContains("not authorized")
                || message.contains("-1743")
                || message.localizedCaseInsensitiveContains("assistive") {
                return "\(browser) automation denied"
            }
            return "\(browser): \(message.trimmingCharacters(in: .whitespacesAndNewlines))"
        default:
            return "\(browser): \(error.localizedDescription)"
        }
    }

    private static func diagnostic(records: [TabRecord], issues: [String]) -> ModuleDiagnostic? {
        guard !issues.isEmpty else { return nil }
        let message = issues.joined(separator: "; ")
        if records.isEmpty,
           issues.contains(where: { $0.localizedCaseInsensitiveContains("automation denied") }) {
            return ModuleDiagnostic(kind: .permissionRequired, message: message)
        }
        if records.isEmpty {
            return ModuleDiagnostic(kind: .error, message: message)
        }
        return ModuleDiagnostic(kind: .degraded, message: message)
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
