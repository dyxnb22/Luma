import Foundation

/// Lightweight running-app metadata collected on the MainActor (no AX / CGWindow IPC).
public struct RunningAppMetadata: Sendable, Equatable {
    public let pid: Int32
    public let bundleID: String
    public let name: String
    public let appURLPath: String

    public init(pid: Int32, bundleID: String, name: String, appURLPath: String) {
        self.pid = pid
        self.bundleID = bundleID
        self.name = name
        self.appURLPath = appURLPath
    }
}
