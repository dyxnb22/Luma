import AppKit
import LumaCore

/// Keeps launcher panel content pinned after in-panel layout changes (search prefixes, results, detail).
@MainActor
enum LauncherInPanelLayout {
    static func stabilizePanel(from view: NSView?) {
        guard let panel = view?.window as? LauncherPanel else { return }
        panel.enforceLockedGeometry()
    }

    /// Re-locks to the current presentation screen when home split is shown (ADR-032).
    static func ensureHomeSplitPanelSize(from view: NSView?) {
        guard let panel = view?.window as? LauncherPanel,
              let screen = LumaPresentationScreen.current() else { return }
        panel.position(on: screen.visibleFrame)
    }
}
