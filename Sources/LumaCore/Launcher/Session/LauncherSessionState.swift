import Foundation

// MARK: - Panel

/// Shadow session reducer for launcher axes (panel / content / detail).
///
/// `LauncherRootController` applies returned `LauncherSessionEffect` values via `LauncherSessionEffectApplier`.
/// Detail search-field suspend/restore is owned by `LauncherDetailPresenter.enterDetailContext` and
/// `exitDetailFromChrome` / `LumaSearchBar.endDetailMode` — not by this reducer.

public enum LauncherPanelPhase: Equatable, Sendable {
    case hidden
    case showing
    case visible
    case hiding
}

// MARK: - Content / detail

public enum LauncherContentPhase: Equatable, Sendable {
    case home
    case results
    case detail(ModuleIdentifier)
}

public enum LauncherDetailModePhase: Equatable, Sendable {
    case inactive
    case active(suspendedQuery: String?)
    case exiting(LauncherDetailExitOutcome)
}

// MARK: - Events

public enum LauncherSessionEvent: Equatable, Sendable {
    case panelShowBegan
    case panelShowCompleted
    case panelHideBegan
    case panelHideCompleted
    case queryBecameEmpty
    case queryBecameNonempty
    case detailOpenRequested(ModuleIdentifier)
    case detailOpened(ModuleIdentifier, suspendedQuery: String?)
    case detailExitRequested(LauncherDetailExitOutcome)
    case detailClosed
    case userTypedInDetail
}

public enum LauncherSessionEffect: Equatable, Sendable {
    case cancelAllTasks
    case clearDetailModeState
}

/// Single reducer for launcher session axes (detail/content/panel). Illegal transitions are no-ops.
public struct LauncherSessionState: Equatable, Sendable {
    public var panel: LauncherPanelPhase
    public var content: LauncherContentPhase
    public var detailMode: LauncherDetailModePhase
    public var panelGeneration: UInt

    public init(
        panel: LauncherPanelPhase = .hidden,
        content: LauncherContentPhase = .home,
        detailMode: LauncherDetailModePhase = .inactive,
        panelGeneration: UInt = 0
    ) {
        self.panel = panel
        self.content = content
        self.detailMode = detailMode
        self.panelGeneration = panelGeneration
    }

    public var isDetailActive: Bool {
        if case .detail = content { return true }
        if case .active = detailMode { return true }
        return false
    }

    public var showingDetail: Bool {
        if case .detail = content { return true }
        return false
    }

    @discardableResult
    public mutating func apply(_ event: LauncherSessionEvent) -> [LauncherSessionEffect] {
        switch event {
        case .panelShowBegan:
            guard panel == .hidden || panel == .hiding else { return [] }
            panel = .showing
            return []

        case .panelShowCompleted:
            guard panel == .showing else { return [] }
            panel = .visible
            panelGeneration &+= 1
            return []

        case .panelHideBegan:
            guard panel == .visible || panel == .showing else { return [] }
            panel = .hiding
            return [.cancelAllTasks]

        case .panelHideCompleted:
            panel = .hidden
            panelGeneration &+= 1
            return []

        case .queryBecameEmpty:
            guard case .results = content else { return [] }
            content = .home
            return []

        case .queryBecameNonempty:
            if case .detail = content { return [] }
            content = .results
            return []

        case .detailOpenRequested(let moduleID):
            guard case .inactive = detailMode else { return [] }
            detailMode = .active(suspendedQuery: nil)
            content = .detail(moduleID)
            return []

        case .detailOpened(let moduleID, let suspended):
            content = .detail(moduleID)
            detailMode = .active(suspendedQuery: suspended)
            return []

        case .detailExitRequested(let outcome):
            guard isDetailActive else { return [] }
            detailMode = .exiting(outcome)
            return []

        case .detailClosed:
            detailMode = .inactive
            if case .detail = content {
                content = .home
            }
            return [.clearDetailModeState]

        case .userTypedInDetail:
            guard isDetailActive else { return [] }
            detailMode = .inactive
            content = .results
            return [.clearDetailModeState]
        }
    }
}
