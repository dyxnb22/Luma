import Foundation

public protocol CurrentProjectClient: Sendable {
    func snapshot() async -> CurrentProjectContext?
}

public struct NoopCurrentProjectClient: CurrentProjectClient {
    public init() {}

    public func snapshot() async -> CurrentProjectContext? { nil }
}

public protocol SelectionSnapshotClient: Sendable {
    func snapshot() async -> String?
}

public struct NoopSelectionSnapshotClient: SelectionSnapshotClient {
    public init() {}

    public func snapshot() async -> String? { nil }
}
