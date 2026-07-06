import AppKit
import Foundation
import LumaCore

public actor PasteboardService: PasteboardClient {
    public init() {}

    public func write(_ string: String) async throws {
        let succeeded = await MainActor.run {
            NSPasteboard.general.clearContents()
            return NSPasteboard.general.setString(string, forType: .string)
        }
        guard succeeded else {
            throw ModuleError.dataUnavailable
        }
    }

    public func writeSecure(_ string: String, clearAfterSeconds: Int) async throws {
        try await write(string)
        let snapshot = await MainActor.run { NSPasteboard.general.changeCount }
        // Snapshot the freshly written change count; clear only if no other app touched the pasteboard since.
        let delay = max(1, clearAfterSeconds)
        Task {
            try? await Task.sleep(for: .seconds(delay))
            await MainActor.run {
                guard NSPasteboard.general.changeCount == snapshot else { return }
                NSPasteboard.general.clearContents()
            }
        }
    }

    public func writeImage(data: Data, pasteboardType: String) async throws {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setData(data, forType: NSPasteboard.PasteboardType(pasteboardType))
        }
    }

    public func writeFileURLs(_ urls: [URL]) async throws {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.writeObjects(urls as [NSURL])
        }
    }

    public func readString() async -> String? {
        await MainActor.run {
            NSPasteboard.general.string(forType: .string)
        }
    }
}
