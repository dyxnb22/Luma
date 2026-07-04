import Foundation

public protocol MenuBarTreeClient: Sendable {
    func staleMenuItemCountForFrontmost() async -> Int
}

public struct NoopMenuBarTreeClient: MenuBarTreeClient {
    public init() {}

    public func staleMenuItemCountForFrontmost() async -> Int { 0 }
}
