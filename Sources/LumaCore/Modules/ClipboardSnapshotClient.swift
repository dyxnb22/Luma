import Foundation

public protocol ClipboardSnapshotClient: Sendable {
    func readSnapshot() async -> ClipboardSnapshot
}

public struct NoopClipboardSnapshotClient: ClipboardSnapshotClient {
    public init() {}

    public func readSnapshot() async -> ClipboardSnapshot {
        ClipboardSnapshot(
            changeCount: 0,
            types: [],
            text: nil,
            imageData: nil,
            imageType: nil,
            fileURLs: [],
            sourceAppName: nil,
            sourceBundleID: nil
        )
    }
}
