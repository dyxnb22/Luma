import AppKit
import Foundation
import LumaCore

public actor RunningApplicationsCache: RunningApplicationsClient {
    public static let shared = RunningApplicationsCache()

    private var cache: Set<String> = []
    private var lastRefresh = Date.distantPast
    private let ttl: TimeInterval
    private var isMonitoring = false
    private var refreshTask: Task<Void, Never>?
    private var launchObserver: NSObjectProtocol?
    private var terminateObserver: NSObjectProtocol?

    internal private(set) var refreshCallCount = 0

    public init(ttl: TimeInterval = 2.0) {
        self.ttl = ttl
    }

    public func startMonitoring() async {
        guard !isMonitoring else { return }
        isMonitoring = true
        installObservers()
        await refresh()
    }

    public func stopMonitoring() async {
        isMonitoring = false
        removeObservers()
        refreshTask?.cancel()
        refreshTask = nil
    }

    public func runningBundleIDs() async -> Set<String> {
        if Date().timeIntervalSince(lastRefresh) > ttl {
            scheduleRefresh()
        }
        return cache
    }

    internal func seedCacheForTesting(_ bundleIDs: Set<String>, lastRefresh: Date = Date()) {
        cache = bundleIDs
        self.lastRefresh = lastRefresh
    }

    internal var hasObserversInstalled: Bool {
        launchObserver != nil || terminateObserver != nil
    }

    private func scheduleRefresh() {
        guard refreshTask == nil else { return }
        refreshTask = Task { [weak self] in
            guard let self else { return }
            await self.refresh()
            await self.clearRefreshTask()
        }
    }

    private func clearRefreshTask() {
        refreshTask = nil
    }

    private func refresh() async {
        refreshCallCount += 1
        let ids = await MainActor.run {
            Set(NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier))
        }
        cache = ids
        lastRefresh = Date()
    }

    private func installObservers() {
        guard launchObserver == nil, terminateObserver == nil else { return }
        let center = NSWorkspace.shared.notificationCenter
        launchObserver = center.addObserver(
            forName: NSWorkspace.didLaunchApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  let bundleID = app.bundleIdentifier else { return }
            Task { await self?.noteApplicationLaunched(bundleID: bundleID) }
        }
        terminateObserver = center.addObserver(
            forName: NSWorkspace.didTerminateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  let bundleID = app.bundleIdentifier else { return }
            Task { await self?.noteApplicationTerminated(bundleID: bundleID) }
        }
    }

    private func removeObservers() {
        let center = NSWorkspace.shared.notificationCenter
        if let launchObserver {
            center.removeObserver(launchObserver)
            self.launchObserver = nil
        }
        if let terminateObserver {
            center.removeObserver(terminateObserver)
            self.terminateObserver = nil
        }
    }

    private func noteApplicationLaunched(bundleID: String) {
        cache.insert(bundleID)
        lastRefresh = Date()
    }

    private func noteApplicationTerminated(bundleID: String) {
        cache.remove(bundleID)
        lastRefresh = Date()
    }
}
