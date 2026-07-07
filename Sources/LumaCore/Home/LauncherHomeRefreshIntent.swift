import Foundation

/// Separates background cache warmup from visible home repaint (Phase 11.5).
public enum LauncherHomeRefreshIntent: Sendable {
    /// Paint empty-query home when guards pass; may close hotkey-home latency when panel is active.
    case visibleRepaint
    /// Fetch home snapshot only — no list paint, selection, rendered-generation advance, or latency samples.
    case backgroundCacheWarm
}

/// Pure repaint / generation / latency policy for `refreshHome(intent:)`.
public enum LauncherHomeRefreshRepaintPolicy {
    public struct VisibleRepaintGuards: Equatable, Sendable {
        public var queryTrimmedEmpty: Bool
        public var showingDetail: Bool
        public var showingResults: Bool
        public var isLauncherQueryEmpty: Bool

        public init(
            queryTrimmedEmpty: Bool,
            showingDetail: Bool,
            showingResults: Bool,
            isLauncherQueryEmpty: Bool
        ) {
            self.queryTrimmedEmpty = queryTrimmedEmpty
            self.showingDetail = showingDetail
            self.showingResults = showingResults
            self.isLauncherQueryEmpty = isLauncherQueryEmpty
        }
    }

    public static func shouldRepaintHome(
        intent: LauncherHomeRefreshIntent,
        guards: VisibleRepaintGuards
    ) -> Bool {
        switch intent {
        case .backgroundCacheWarm:
            return false
        case .visibleRepaint:
            guard guards.queryTrimmedEmpty else { return false }
            if guards.showingDetail { return true }
            return !guards.showingResults && guards.isLauncherQueryEmpty
        }
    }

    /// `lastRenderedHomeGeneration` tracks UI paint completion, not cache warmup.
    public static func shouldAdvanceRenderedGeneration(
        intent: LauncherHomeRefreshIntent,
        didRepaint: Bool
    ) -> Bool {
        intent == .visibleRepaint && didRepaint
    }

    public static func shouldCloseHotkeyLatencyOnCacheHit(
        intent: LauncherHomeRefreshIntent,
        isPanelActiveForQueryApply: Bool
    ) -> Bool {
        intent == .visibleRepaint && isPanelActiveForQueryApply
    }
}
