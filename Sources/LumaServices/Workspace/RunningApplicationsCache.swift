import AppKit
import Foundation
import LumaCore

public actor RunningApplicationsCache: RunningApplicationsClient {
    public static let shared = RunningApplicationsCache()

    private var cache: Set<String> = []
    private var lastRefresh = Date.distantPast
    private let ttl: TimeInterval
    private var isMonitoring = false

    public init(ttl: TimeInterval = 2.0) {
        self.ttl = ttl
    }

    public func startMonitoring() async {
        guard !isMonitoring else { return }
        isMonitoring = true
        await refresh()
    }

    public func stopMonitoring() async {
        isMonitoring = false
    }

    public func runningBundleIDs() async -> Set<String> {
        if Date().timeIntervalSince(lastRefresh) > ttl {
            await refresh()
        }
        return cache
    }

    private func refresh() async {
        let ids = await MainActor.run {
            Set(NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier))
        }
        cache = ids
        lastRefresh = Date()
    }
}
