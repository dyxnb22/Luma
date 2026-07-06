import Foundation

public protocol WorkspaceClient: Sendable {
    func launchApplication(at url: URL) async throws
    func openURL(_ url: URL) async throws
    func openLocalFileURL(_ url: URL) async throws
    func revealInFinder(_ url: URL) async throws
    func terminateApplication(bundleID: String) async
    func openApplication(bundleID: String, arguments: [String]) async
}

public struct NoopWorkspaceClient: WorkspaceClient {
    public init() {}

    public func launchApplication(at url: URL) async throws {
        _ = url
    }

    public func openURL(_ url: URL) async throws {
        _ = url
    }

    public func openLocalFileURL(_ url: URL) async throws {
        _ = url
    }

    public func revealInFinder(_ url: URL) async throws {
        _ = url
    }

    public func terminateApplication(bundleID: String) async {
        _ = bundleID
    }

    public func openApplication(bundleID: String, arguments: [String]) async {
        _ = bundleID
        _ = arguments
    }
}
