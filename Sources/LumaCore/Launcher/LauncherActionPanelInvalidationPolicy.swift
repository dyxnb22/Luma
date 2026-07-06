import Foundation

/// Action panel invalidation when the search query changes.
public enum LauncherActionPanelInvalidationPolicy {
    /// Returns true when a query edit should dismiss a visible action panel.
    public static func shouldDismissOnQueryChange(
        previousQuery: String,
        newQuery: String,
        actionPanelVisible: Bool
    ) -> Bool {
        guard previousQuery != newQuery else { return false }
        return actionPanelVisible
    }
}
