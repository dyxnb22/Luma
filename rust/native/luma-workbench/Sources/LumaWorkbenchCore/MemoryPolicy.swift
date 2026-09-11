import Foundation

public enum MemoryPressureLevel: Equatable {
    case warning
    case critical
}

public struct ResidentMemorySnapshot: Equatable {
    public let hostBytes: UInt64
    public let childBytes: UInt64

    public init(hostBytes: UInt64, childBytes: UInt64) {
        self.hostBytes = hostBytes
        self.childBytes = childBytes
    }

    public var totalBytes: UInt64 { hostBytes.saturatingAdding(childBytes) }
}

public struct ResidentMemoryTracker: Equatable {
    public private(set) var latest: ResidentMemorySnapshot?
    public private(set) var peakTotalBytes: UInt64 = 0

    public init() {}

    public mutating func record(_ snapshot: ResidentMemorySnapshot) {
        latest = snapshot
        peakTotalBytes = max(peakTotalBytes, snapshot.totalBytes)
    }
}

public enum MemoryPressurePolicy {
    public static let samplingInterval: TimeInterval = 30
    public static let warningScrollbackLines = 250
    public static let criticalScrollbackLines = 50

    public static func scrollbackLines(for level: MemoryPressureLevel) -> Int {
        switch level {
        case .warning: warningScrollbackLines
        case .critical: criticalScrollbackLines
        }
    }
}

private extension UInt64 {
    func saturatingAdding(_ other: UInt64) -> UInt64 {
        let (sum, overflow) = addingReportingOverflow(other)
        return overflow ? .max : sum
    }
}
