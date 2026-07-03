import AppKit

/// Chooses which display to use when presenting transient Luma windows (launcher, Settings).
@MainActor
enum LumaPresentationScreen {
    /// Screen under the cursor, else the screen showing the key window, else main.
    static func current() -> NSScreen? {
        let mouse = NSEvent.mouseLocation
        if let underMouse = NSScreen.screens.first(where: { $0.frame.contains(mouse) }) {
            return underMouse
        }
        if let key = NSApp.keyWindow?.screen {
            return key
        }
        return NSScreen.main ?? NSScreen.screens.first
    }
}
