import Foundation

public struct LauncherSearchDetailModeState: Equatable, Sendable {
    public var visibleQuery: String
    public var suspendedQuery: String?
    public var isEditable: Bool

    public init(visibleQuery: String = "", suspendedQuery: String? = nil, isEditable: Bool = true) {
        self.visibleQuery = visibleQuery
        self.suspendedQuery = suspendedQuery
        self.isEditable = isEditable
    }
}

/// Pure detail-mode transitions for the launcher search field.
public enum LauncherSearchDetailMode {
    public static func beginDetailMode(
        _ state: LauncherSearchDetailModeState,
        moduleTitle: String
    ) -> LauncherSearchDetailModeState {
        var next = state
        if next.suspendedQuery == nil {
            next.suspendedQuery = next.visibleQuery
        }
        next.visibleQuery = ""
        next.isEditable = false
        _ = moduleTitle
        return next
    }

    public static func endDetailMode(
        _ state: LauncherSearchDetailModeState
    ) -> (state: LauncherSearchDetailModeState, restoredQuery: String?) {
        var next = state
        next.isEditable = true
        let restored = next.suspendedQuery
        next.suspendedQuery = nil
        return (next, restored)
    }

    public static func cancelDetailMode(_ state: LauncherSearchDetailModeState) -> LauncherSearchDetailModeState {
        var next = state
        next.suspendedQuery = nil
        next.isEditable = true
        return next
    }

    public static func clearStuckDetailModeState(_ state: LauncherSearchDetailModeState) -> LauncherSearchDetailModeState {
        var next = state
        next.suspendedQuery = nil
        next.isEditable = true
        return next
    }

    public static func reEnableSearchFieldIfNeeded(_ state: LauncherSearchDetailModeState) -> LauncherSearchDetailModeState {
        guard !state.isEditable else { return state }
        var next = state
        next.isEditable = true
        return next
    }
}
