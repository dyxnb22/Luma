import AppKit
import LumaCore
import LumaInfrastructure

/// Keeps launcher panel content pinned after in-panel layout changes (search prefixes, results, detail).
@MainActor
enum LauncherInPanelLayout {
    private static weak var pendingStabilizeView: NSView?
    private static var coalesceScheduled = false
    private static var lastPositionedScreenID: CGDirectDisplayID?

    static func stabilizePanel(from view: NSView?) {
        pendingStabilizeView = view
        guard !coalesceScheduled else { return }
        coalesceScheduled = true
        Task { @MainActor in
            coalesceScheduled = false
            let view = pendingStabilizeView
            pendingStabilizeView = nil
            stabilizePanelImmediately(from: view)
        }
    }

    private static func stabilizePanelImmediately(from view: NSView?) {
        LauncherPerfCounters.increment(.layoutPanel)
        guard let panel = view?.window as? LauncherPanel else { return }
        panel.enforceLockedGeometry()
    }

    /// Re-locks to the current presentation screen when home split is shown (ADR-032).
    static func ensureHomeSplitPanelSize(from view: NSView?) {
        guard let panel = view?.window as? LauncherPanel,
              let screen = LumaPresentationScreen.current() else { return }
        let screenID = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID
        if let screenID, screenID == lastPositionedScreenID { return }
        lastPositionedScreenID = screenID
        panel.position(on: screen.visibleFrame)
    }
}
