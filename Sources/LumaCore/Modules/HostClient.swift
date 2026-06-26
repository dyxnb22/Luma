import Foundation

public protocol HostClient: Sendable {
    func openSettings() async
    func reloadModules() async
    func quitHost() async
}

public struct NoopHostClient: HostClient {
    public init() {}

    public func openSettings() async {}
    public func reloadModules() async {}
    public func quitHost() async {}
}
