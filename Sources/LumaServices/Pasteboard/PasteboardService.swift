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
}
