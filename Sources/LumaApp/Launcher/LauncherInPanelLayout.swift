import AppKit
import LumaCore

/// Keeps launcher panel content pinned after in-panel layout changes (search prefixes, results, detail).
@MainActor
enum LauncherInPanelLayout {
    static func stabilizePanel(from view: NSView?) {
        guard let panel = view?.window as? LauncherPanel else { return }
        panel.enforceLockedGeometry()
    }
}
