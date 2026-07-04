import Foundation

public enum LauncherSplitRightPane: Equatable, Sendable {
    case guide
    case detail
    case hidden
}

public struct LauncherHomeSplitState: Equatable, Sendable {
    public let columnSplitActive: Bool
    public let rightPane: LauncherSplitRightPane

    public init(columnSplitActive: Bool, rightPane: LauncherSplitRightPane) {
        self.columnSplitActive = columnSplitActive
        self.rightPane = rightPane
    }
}

/// Pure layout planner for empty-query home split (Open Apps left + guide/detail right). ADR-032.
public enum LauncherHomeSplitPlanner {
    public static func layout(
        queryTrimmedIsEmpty: Bool,
        showingDetail: Bool,
        showingResults: Bool
    ) -> LauncherHomeSplitState {
        let columnSplit: Bool
        if !queryTrimmedIsEmpty {
            columnSplit = false
        } else if showingDetail {
            columnSplit = true
        } else {
            columnSplit = !showingResults
        }

        let rightPane: LauncherSplitRightPane
        if columnSplit, showingDetail {
            rightPane = .detail
        } else if columnSplit {
            rightPane = .guide
        } else {
            rightPane = .hidden
        }

        return LauncherHomeSplitState(columnSplitActive: columnSplit, rightPane: rightPane)
    }
}
