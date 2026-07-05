import Foundation

public struct LauncherHomeSplitConstraintFlags: Equatable, Sendable {
    public let listSplitWidthActive: Bool
    public let listFullWidthTrailingActive: Bool
    public let detailRightColumnActive: Bool
    public let guideVisible: Bool
    public let detailVisible: Bool
    public let dividerVisible: Bool

    public init(
        listSplitWidthActive: Bool,
        listFullWidthTrailingActive: Bool,
        detailRightColumnActive: Bool,
        guideVisible: Bool,
        detailVisible: Bool,
        dividerVisible: Bool
    ) {
        self.listSplitWidthActive = listSplitWidthActive
        self.listFullWidthTrailingActive = listFullWidthTrailingActive
        self.detailRightColumnActive = detailRightColumnActive
        self.guideVisible = guideVisible
        self.detailVisible = detailVisible
        self.dividerVisible = dividerVisible
    }
}

/// Maps split state to AppKit constraint and visibility flags (ADR-032).
public enum LauncherHomeSplitConstraintPolicy {
    public static func flags(for state: LauncherHomeSplitState) -> LauncherHomeSplitConstraintFlags {
        let columnSplit = state.columnSplitActive
        let showsDetail = state.rightPane == .detail
        return LauncherHomeSplitConstraintFlags(
            listSplitWidthActive: columnSplit,
            listFullWidthTrailingActive: !columnSplit,
            detailRightColumnActive: columnSplit && showsDetail,
            guideVisible: columnSplit && state.rightPane == .guide,
            detailVisible: showsDetail,
            dividerVisible: columnSplit
        )
    }
}
