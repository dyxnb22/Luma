import Foundation

// MARK: - Snapshot

public struct LauncherStateSnapshot: Codable, Equatable, Sendable {
    public struct PanelState: Codable, Equatable, Sendable {
        public let panelVisible: Bool
        public let visibilitySessionVisible: Bool
        public let visibilityGeneration: UInt
        public let isKeyWindow: Bool
        public let firstResponderChain: String

        public init(
            panelVisible: Bool,
            visibilitySessionVisible: Bool,
            visibilityGeneration: UInt,
            isKeyWindow: Bool,
            firstResponderChain: String
        ) {
            self.panelVisible = panelVisible
            self.visibilitySessionVisible = visibilitySessionVisible
            self.visibilityGeneration = visibilityGeneration
            self.isKeyWindow = isKeyWindow
            self.firstResponderChain = firstResponderChain
        }
    }

    public struct SearchState: Codable, Equatable, Sendable {
        public let visibleQuery: String
        public let persistedQuery: String
        public let isDetailModeActive: Bool
        public let isEditable: Bool
        public let placeholder: String?

        public init(
            visibleQuery: String,
            persistedQuery: String,
            isDetailModeActive: Bool,
            isEditable: Bool,
            placeholder: String?
        ) {
            self.visibleQuery = visibleQuery
            self.persistedQuery = persistedQuery
            self.isDetailModeActive = isDetailModeActive
            self.isEditable = isEditable
            self.placeholder = placeholder
        }
    }

    public enum ContentModeKind: String, Codable, Sendable {
        case home
        case results
        case detail
    }

    public struct ContentState: Codable, Equatable, Sendable {
        public let modeKind: ContentModeKind
        public let detailModuleID: String?
        public let showingDetail: Bool
        public let showingResults: Bool
        public let selectedIndex: Int
        public let selectedItemID: String?
        public let currentDetailModuleID: String?

        public init(
            modeKind: ContentModeKind,
            detailModuleID: String?,
            showingDetail: Bool,
            showingResults: Bool,
            selectedIndex: Int,
            selectedItemID: String?,
            currentDetailModuleID: String?
        ) {
            self.modeKind = modeKind
            self.detailModuleID = detailModuleID
            self.showingDetail = showingDetail
            self.showingResults = showingResults
            self.selectedIndex = selectedIndex
            self.selectedItemID = selectedItemID
            self.currentDetailModuleID = currentDetailModuleID
        }
    }

    public enum SplitRightPaneKind: String, Codable, Sendable {
        case guide
        case detail
        case hidden
    }

    public struct ChromeState: Codable, Equatable, Sendable {
        public let detailContainerHidden: Bool
        public let detailContainerAlpha: Double
        public let splitColumnActive: Bool
        public let splitRightPane: SplitRightPaneKind
        public let homeVisible: Bool
        public let resultsVisible: Bool
        public let hintContext: String

        public init(
            detailContainerHidden: Bool,
            detailContainerAlpha: Double,
            splitColumnActive: Bool,
            splitRightPane: SplitRightPaneKind,
            homeVisible: Bool,
            resultsVisible: Bool,
            hintContext: String
        ) {
            self.detailContainerHidden = detailContainerHidden
            self.detailContainerAlpha = detailContainerAlpha
            self.splitColumnActive = splitColumnActive
            self.splitRightPane = splitRightPane
            self.homeVisible = homeVisible
            self.resultsVisible = resultsVisible
            self.hintContext = hintContext
        }
    }

    public struct AnimationState: Codable, Equatable, Sendable {
        public let detailPresentationGeneration: UInt
        public let crossfadeGeneration: UInt
        public let detailCloseCrossfadeInFlight: Bool

        public init(
            detailPresentationGeneration: UInt,
            crossfadeGeneration: UInt,
            detailCloseCrossfadeInFlight: Bool
        ) {
            self.detailPresentationGeneration = detailPresentationGeneration
            self.crossfadeGeneration = crossfadeGeneration
            self.detailCloseCrossfadeInFlight = detailCloseCrossfadeInFlight
        }
    }

    public let generatedAt: String
    public let reason: String?
    public let panel: PanelState
    public let search: SearchState
    public let content: ContentState
    public let chrome: ChromeState
    public let animation: AnimationState
    public let lastKeyboardCommand: String?
    public let searchFieldCanBecomeFirstResponder: Bool

    public init(
        generatedAt: String,
        reason: String?,
        panel: PanelState,
        search: SearchState,
        content: ContentState,
        chrome: ChromeState,
        animation: AnimationState,
        lastKeyboardCommand: String?,
        searchFieldCanBecomeFirstResponder: Bool
    ) {
        self.generatedAt = generatedAt
        self.reason = reason
        self.panel = panel
        self.search = search
        self.content = content
        self.chrome = chrome
        self.animation = animation
        self.lastKeyboardCommand = lastKeyboardCommand
        self.searchFieldCanBecomeFirstResponder = searchFieldCanBecomeFirstResponder
    }
}

// MARK: - Keyboard command recorder

public enum LauncherStateKeyboardRecorder {
    private static let lock = NSLock()
    nonisolated(unsafe) private static var _lastCommand: String?

    public static var lastCommand: String? {
        lock.lock()
        defer { lock.unlock() }
        return _lastCommand
    }

    public static func record(_ command: String) {
        lock.lock()
        defer { lock.unlock() }
        _lastCommand = command
    }
}

// MARK: - Invariants

public enum LauncherStateInvariant: String, Codable, Sendable, CaseIterable {
    case detailModeWithoutContentDetail = "I1_detailModeWithoutContentDetail"
    case detailModuleIDWithoutVisibleContainer = "I2_detailModuleIDWithoutVisibleContainer"
    case visibleDetailContainerWrongSplitPane = "I3_visibleDetailContainerWrongSplitPane"
    case panelVisibleWithoutFocusFallback = "I4_panelVisibleWithoutFocusFallback"
    case reopenDetailPlaceholderWithGuidePane = "I5_reopenDetailPlaceholderWithGuidePane"
    case detailContentWithGuideSplitPane = "I6_detailContentWithGuideSplitPane"
}

public struct LauncherStateViolation: Codable, Equatable, Sendable {
    public let timestamp: String
    public let invariant: LauncherStateInvariant
    public let snapshot: LauncherStateSnapshot

    public init(timestamp: String, invariant: LauncherStateInvariant, snapshot: LauncherStateSnapshot) {
        self.timestamp = timestamp
        self.invariant = invariant
        self.snapshot = snapshot
    }
}

public enum LauncherStateInvariantChecker {
    public static func check(_ snapshot: LauncherStateSnapshot) -> [LauncherStateInvariant] {
        var violations: [LauncherStateInvariant] = []

        if snapshot.search.isDetailModeActive, !snapshot.content.showingDetail {
            violations.append(.detailModeWithoutContentDetail)
        }

        if snapshot.content.currentDetailModuleID != nil,
           snapshot.chrome.detailContainerHidden,
           snapshot.chrome.splitRightPane != .detail {
            violations.append(.detailModuleIDWithoutVisibleContainer)
        }

        let detailContainerVisible = !snapshot.chrome.detailContainerHidden
            && snapshot.chrome.detailContainerAlpha > 0.01
        if detailContainerVisible, snapshot.chrome.splitRightPane != .detail {
            violations.append(.visibleDetailContainerWrongSplitPane)
        }

        if snapshot.panel.visibilitySessionVisible,
           !snapshot.searchFieldCanBecomeFirstResponder,
           !snapshot.search.isDetailModeActive,
           !snapshot.content.showingDetail {
            violations.append(.panelVisibleWithoutFocusFallback)
        }

        if snapshot.panel.visibilitySessionVisible,
           snapshot.search.isDetailModeActive,
           !snapshot.content.showingDetail,
           snapshot.chrome.splitColumnActive,
           snapshot.chrome.splitRightPane == .guide {
            violations.append(.reopenDetailPlaceholderWithGuidePane)
        }

        if snapshot.content.showingDetail,
           snapshot.chrome.splitColumnActive,
           snapshot.chrome.splitRightPane == .guide {
            violations.append(.detailContentWithGuideSplitPane)
        }

        return violations
    }
}

public extension LauncherSplitRightPane {
    var snapshotKind: LauncherStateSnapshot.SplitRightPaneKind {
        switch self {
        case .guide: .guide
        case .detail: .detail
        case .hidden: .hidden
        }
    }
}

public extension LauncherContentMode {
    var snapshotKind: LauncherStateSnapshot.ContentModeKind {
        switch self {
        case .home: .home
        case .results: .results
        case .detail: .detail
        }
    }
}
