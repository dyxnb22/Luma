import AppKit
import Foundation

public enum NotesTypora {
    public static func open(_ url: URL) {
        let candidates = [
            URL(fileURLWithPath: "/Applications/Typora.app"),
            FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Applications/Typora.app")
        ]
        if let appURL = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) {
            let configuration = NSWorkspace.OpenConfiguration()
            NSWorkspace.shared.open([url], withApplicationAt: appURL, configuration: configuration) { _, _ in }
        } else {
            NSWorkspace.shared.open(url)
        }
    }
}
