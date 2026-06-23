import AppKit
import LumaModules

/// Presents Notes auxiliary sheets from the detail view.
@MainActor
enum NotesDetailSheets {
    static func presentImageTools(on window: NSWindow, root: URL) async {
        let panel = NotesImageToolsPanel(root: root)
        let sheet = NSWindow(contentViewController: panel)
        await window.beginSheet(sheet)
    }
}
