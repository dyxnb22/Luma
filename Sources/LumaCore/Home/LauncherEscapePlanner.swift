import Foundation

/// Next user-facing step when Esc is pressed in the launcher panel.
public enum LauncherEscapeStep: Sendable, Equatable {
    case dismissActionPanel
    /// Try detail subview Esc handling first; if unhandled, call `exitDetailFromChrome()`.
    case detailEscapeOrExit
    case showHome
    case dismissPanel
}

/// Pure Esc stack for Route C (action panel → detail → home → dismiss panel).
public enum LauncherEscapePlanner {
    public static func nextStep(
        actionPanelVisible: Bool,
        showingDetail: Bool,
        detailContextActive: Bool = false,
        showingResults: Bool,
        queryTrimmedIsEmpty: Bool
    ) -> LauncherEscapeStep {
        if actionPanelVisible {
            return .dismissActionPanel
        }
        if showingDetail || detailContextActive {
            return .detailEscapeOrExit
        }
        if showingResults || !queryTrimmedIsEmpty {
            return .showHome
        }
        return .dismissPanel
    }
}
