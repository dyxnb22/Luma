import Foundation

/// C-UI-004 selection resolution after snapshot/list row changes.
public enum LauncherListSelectionPreservePolicy {
    public static func nextFlatIndex(
        preserveSelection: Bool,
        previousFlatIndex: Int,
        previousItemID: ResultID?,
        selectable: [ResultItem]
    ) -> Int {
        if let previousItemID,
           let restored = selectable.firstIndex(where: { $0.id == previousItemID }) {
            return restored
        }
        if preserveSelection, !selectable.isEmpty {
            return min(previousFlatIndex, selectable.count - 1)
        }
        return 0
    }

    public static func clampedFlatIndex(_ index: Int, selectableCount: Int) -> Int {
        guard selectableCount > 0 else { return 0 }
        return min(max(0, index), selectableCount - 1)
    }
}

/// Return-key activation guard when list selection may be stale after snapshot apply.
public enum LauncherReturnActivationPolicy {
    public enum Outcome: Equatable, Sendable {
        case activateSelected
        case showEmptyQueryMessage
        case showNoResultsYet
    }

    public static func outcome(itemCount: Int, selectedIndex: Int) -> Outcome {
        if itemCount == 0 { return .showEmptyQueryMessage }
        guard selectedIndex >= 0, selectedIndex < itemCount else { return .showNoResultsYet }
        return .activateSelected
    }
}
