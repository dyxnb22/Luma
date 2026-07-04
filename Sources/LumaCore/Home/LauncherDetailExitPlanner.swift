import Foundation

public enum LauncherDetailExitOutcome: Equatable, Sendable {
    case reenableSearchOnly
    case restoreSuspendedQuery(String)
    case returnToHome(crossfadeToGuide: Bool)
}

/// Pure planner for Esc/back/close detail exit — mirrors `exitDetailFromChrome()` routing.
public enum LauncherDetailExitPlanner {
    public static func outcome(
        showingDetail: Bool,
        suspendedQuery: String?,
        columnSplitActive: Bool
    ) -> LauncherDetailExitOutcome {
        guard showingDetail else {
            return .reenableSearchOnly
        }
        if let suspendedQuery,
           !suspendedQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return .restoreSuspendedQuery(suspendedQuery)
        }
        return .returnToHome(crossfadeToGuide: columnSplitActive)
    }
}
