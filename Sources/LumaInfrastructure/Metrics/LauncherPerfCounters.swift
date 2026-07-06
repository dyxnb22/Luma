import Foundation
import LumaCore

public enum LauncherPerfCounters {
    public enum Key: String, CaseIterable, Sendable {
        case layoutPanel = "layout.panel"
        case layoutHint = "layout.hint"
        case permissionRefresh = "permission.refresh"
        case sessionPersist = "session.persist"
        case snapshotApply = "snapshot.apply"
        case detailViewMade = "detail.viewMade"
        case homeSnapshot = "home.snapshot"
        case openAppsRefresh = "openApps.refresh"
        case snapshotApplyCoalesced = "snapshot.applyCoalesced"
        case detailOpen = "detail.open"
        case backHome = "back.home"
    }

    private static let lock = NSLock()
    private nonisolated(unsafe) static var counts: [Key: Int] = [:]

    public static func increment(_ key: Key) {
        lock.lock()
        counts[key, default: 0] += 1
        lock.unlock()
    }

    public static func count(for key: Key) -> Int {
        lock.lock()
        defer { lock.unlock() }
        return counts[key, default: 0]
    }

    public static func reset() {
        lock.lock()
        counts.removeAll(keepingCapacity: true)
        lock.unlock()
    }

    public static func exportSnapshot() -> [String: Int] {
        lock.lock()
        defer { lock.unlock() }
        return Dictionary(uniqueKeysWithValues: Key.allCases.map { ($0.rawValue, counts[$0, default: 0]) })
    }
}
