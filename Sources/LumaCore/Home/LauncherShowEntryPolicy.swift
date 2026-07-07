import Foundation

/// Why the launcher panel is being shown — used to document entry semantics (Phase 12.2–12.3).
public enum LauncherShowReason: Sendable {
    case carbonHotkey
    case menuBar
    case restore
    case qa
}

/// Pure policy for show-entry guards. Production routes through `LauncherWindowController`.
public enum LauncherShowEntryPolicy {
    /// Carbon hotkey is hidden-only; menu bar / restore / QA may re-show while already visible.
    public static func shouldBeginShowWhenAlreadyVisible(reason: LauncherShowReason) -> Bool {
        switch reason {
        case .carbonHotkey:
            return false
        case .menuBar, .restore, .qa:
            return true
        }
    }

    public static func appliesCarbonShowDebounce(reason: LauncherShowReason) -> Bool {
        reason == .carbonHotkey
    }
}
