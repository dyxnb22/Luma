import Foundation

public struct MatchedProject: Sendable, Equatable {
    public let path: String
    public let name: String

    public init(path: String, name: String) {
        self.path = path
        self.name = name
    }
}

public protocol ProjectMatcherClient: Sendable {
    func match(label: String) async -> MatchedProject?
}

public struct NoopProjectMatcherClient: ProjectMatcherClient {
    public init() {}

    public func match(label: String) async -> MatchedProject? {
        _ = label
        return nil
    }
}
