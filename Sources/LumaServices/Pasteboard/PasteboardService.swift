import AppKit
import Foundation
import LumaCore

public actor PasteboardService: PasteboardClient {
    public init() {}

    public func write(_ string: String) async {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(string, forType: .string)
        }
    }

    public func writeSecure(_ string: String, clearAfterSeconds: Int) async {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(string, forType: .string)
        }
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

    public func writeImage(data: Data, pasteboardType: String) async {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setData(data, forType: NSPasteboard.PasteboardType(pasteboardType))
        }
    }
}
