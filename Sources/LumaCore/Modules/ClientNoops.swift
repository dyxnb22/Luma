import Foundation

public struct NoopPasteboardClient: PasteboardClient {
    public init() {}

    public func write(_ string: String) async { _ = string }
    public func writeSecure(_ string: String, clearAfterSeconds: Int) async {
        _ = string
        _ = clearAfterSeconds
    }
    public func writeImage(data: Data, pasteboardType: String) async {
        _ = data
        _ = pasteboardType
    }
    public func writeFileURLs(_ urls: [URL]) async { _ = urls }
    public func readString() async -> String? { nil }
}

public struct NoopAccessibilityClient: AccessibilityClient {
    public init() {}

    public func isTrusted() async -> Bool { false }
    public func requestPermission() async {}

    public func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {
        _ = windowID
        _ = pid
        _ = title
        _ = axTitle
        _ = bounds
    }

    public func insert(text: String) async { _ = text }
    public func applyWindowLayout(_ preset: String) async { _ = preset }
}

public struct NoopFileSystemClient: FileSystemClient {
    public init() {}

    public func watch(root: URL, debounceMillis: Int) async -> AsyncStream<[FSChangeEvent]> {
        _ = root
        _ = debounceMillis
        return AsyncStream { $0.finish() }
    }

    public func stopWatching(root: URL) async { _ = root }
}
