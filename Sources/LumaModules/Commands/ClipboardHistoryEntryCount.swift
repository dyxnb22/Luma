import Foundation
import LumaCore

/// Reads clipboard history entry count for doctor diagnostics.
enum ClipboardHistoryEntryCount {
    static func readFromApplicationSupport(fileManager: FileManager = .default) -> Int? {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        let url = base.appendingPathComponent("Luma/clipboard-history.json")
        guard let data = try? Data(contentsOf: url),
              let entries = try? JSONDecoder().decode([ClipboardEntry].self, from: data) else {
            return nil
        }
        return entries.count
    }
}
