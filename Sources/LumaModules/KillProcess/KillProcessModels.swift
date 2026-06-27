import Foundation
import LumaCore
import LumaServices

public struct KillProcessMatch: Sendable, Hashable {
    public let record: RunningProcessRecord
    public let score: Double

    public init(record: RunningProcessRecord, score: Double) {
        self.record = record
        self.score = score
    }
}

public enum KillProcessIndex {
    public static let guardedBundleIDs: Set<String> = [
        "com.apple.dock",
        "com.apple.finder",
        "com.apple.WindowServer",
        "com.apple.systemuiserver"
    ]

    public static func filtered(_ records: [RunningProcessRecord], selfPID: pid_t = ProcessInfo.processInfo.processIdentifier) -> [RunningProcessRecord] {
        records.filter { $0.pid != selfPID }
    }

    public static func recent(_ records: [RunningProcessRecord], limit: Int = 8) -> [RunningProcessRecord] {
        filtered(records)
            .sorted { lhs, rhs in
                switch (lhs.launchDate, rhs.launchDate) {
                case let (l?, r?): return l > r
                case (_?, nil): return true
                case (nil, _?): return false
                case (nil, nil): return lhs.name.localizedCaseInsensitiveCompare(rhs.name) == .orderedAscending
                }
            }
            .prefix(limit)
            .map { $0 }
    }

    public static func search(_ records: [RunningProcessRecord], query: String, limit: Int = 8) -> [KillProcessMatch] {
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !q.isEmpty else {
            return recent(records, limit: limit).map { KillProcessMatch(record: $0, score: 1) }
        }
        return filtered(records).compactMap { record -> KillProcessMatch? in
            let target = "\(record.name) \(record.bundleID)".lowercased()
            let score = FuzzyMatcher.score(query: q, target: target)
            guard score > 0 else { return nil }
            return KillProcessMatch(record: record, score: score)
        }
        .sorted { $0.score > $1.score }
        .prefix(limit)
        .map { $0 }
    }

    public static func memoryDisplay(bytes: UInt64?) -> String {
        guard let bytes else { return "memory unknown" }
        let mb = Double(bytes) / 1_048_576.0
        if mb >= 1024 {
            return String(format: "%.1f GB", mb / 1024.0)
        }
        return String(format: "%.0f MB", mb)
    }
}
