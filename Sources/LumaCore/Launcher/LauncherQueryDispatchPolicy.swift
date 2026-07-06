import Foundation

/// Whether launcher query dispatch should run while IME composition is active.
public enum LauncherQueryDispatchPolicy {
    /// Returns false while marked text is active so query dispatch waits for commit.
    public static func shouldDispatchQuery(isComposing: Bool) -> Bool {
        !isComposing
    }
}
