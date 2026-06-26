import Foundation

public struct RunningApplicationMemory: Sendable, Hashable {
    public let bundleID: String
    public let name: String
    public let residentBytes: UInt64

    public var residentMB: Double {
        Double(residentBytes) / 1_048_576.0
    }

    public init(bundleID: String, name: String, residentBytes: UInt64) {
        self.bundleID = bundleID
        self.name = name
        self.residentBytes = residentBytes
    }
}

public protocol ProcessMemoryClient: Sendable {
    func topApplications(limit: Int) async -> [RunningApplicationMemory]
}

public struct NoopProcessMemoryClient: ProcessMemoryClient {
    public init() {}

    public func topApplications(limit: Int) async -> [RunningApplicationMemory] {
        _ = limit
        return []
    }
}
