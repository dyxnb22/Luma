import Foundation

public struct NoopPasteboardClient: PasteboardClient {
    public init() {}

    public func write(_ string: String) async throws { _ = string }
    public func writeSecure(_ string: String, clearAfterSeconds: Int) async throws {
        _ = string
        _ = clearAfterSeconds
    }
    public func writeImage(data: Data, pasteboardType: String) async throws {
        _ = data
        _ = pasteboardType
    }
    public func writeFileURLs(_ urls: [URL]) async throws { _ = urls }
    public func readString() async -> String? { nil }
}

public struct NoopAccessibilityClient: AccessibilityClient {
    public init() {}

    public func isTrusted() async -> Bool { false }
    public func requestPermission() async {}

    public func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async throws {
        _ = windowID
        _ = pid
        _ = title
        _ = axTitle
        _ = bounds
        throw ModuleError.permissionRequired(.accessibility)
    }

    public func insert(text: String) async throws {
        _ = text
        throw ModuleError.permissionRequired(.accessibility)
    }
    public func replaceSelectedText(with text: String) async -> Bool { _ = text; return false }
    public func applyWindowLayout(_ preset: String) async throws {
        _ = preset
        throw ModuleError.permissionRequired(.accessibility)
    }
}

public struct NoopTranslationClient: TranslationClient {
    public init() {}

    public func translate(_ text: String) async throws -> TranslationOutcome {
        TranslationOutcome(text: text)
    }
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
