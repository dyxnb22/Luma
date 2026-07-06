import CoreGraphics
import Foundation

/// Pure policy for repositioning the launcher panel when the presentation screen changes.
public enum LauncherPanelRepositionPolicy {
    /// Returns true when the panel should be repositioned on a screen/Space change.
    public static func shouldReposition(
        isPanelVisible: Bool,
        lastVisibleFrame: CGRect?,
        newVisibleFrame: CGRect
    ) -> Bool {
        guard isPanelVisible else { return false }
        if let last = lastVisibleFrame, last.equalTo(newVisibleFrame) {
            return false
        }
        return true
    }
}
