import Foundation

public protocol HostClient: Sendable {
    func openSettings() async
    func reloadModules() async
    func quitHost() async
    func exportDiagnostics() async throws -> URL
}

public struct NoopHostClient: HostClient {
    public init() {}

    public func openSettings() async {}
    public func reloadModules() async {}
    public func quitHost() async {}
    public func exportDiagnostics() async throws -> URL {
        throw NSError(domain: "HostClient", code: 1, userInfo: [NSLocalizedDescriptionKey: "exportDiagnostics unavailable"])
    }
}
