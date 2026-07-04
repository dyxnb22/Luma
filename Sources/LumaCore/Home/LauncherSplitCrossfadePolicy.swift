import Foundation

public enum LauncherSplitCrossfadeDirection: Equatable, Sendable {
    case detailToGuide
    case guideToDetail
}

public struct LauncherOverlayHitTestState: Equatable, Sendable {
    public var passesHitTests: Bool
    public var isHidden: Bool
    public var alphaVisible: Bool

    public init(passesHitTests: Bool, isHidden: Bool, alphaVisible: Bool) {
        self.passesHitTests = passesHitTests
        self.isHidden = isHidden
        self.alphaVisible = alphaVisible
    }

    /// Mirrors `LauncherOverlayHostView` / `LauncherHomeGuidePane` hit-test guards.
    public var acceptsMouseHits: Bool {
        passesHitTests && alphaVisible && !isHidden
    }
}

public struct LauncherSplitCrossfadePaneState: Equatable, Sendable {
    public let guide: LauncherOverlayHitTestState
    public let detail: LauncherOverlayHitTestState

    public init(guide: LauncherOverlayHitTestState, detail: LauncherOverlayHitTestState) {
        self.guide = guide
        self.detail = detail
    }
}

/// Expected overlay hit-test state during and after guide ↔ detail cross-fades (audit L1).
public enum LauncherSplitCrossfadePolicy {
    public static func duringAnimation(_ direction: LauncherSplitCrossfadeDirection) -> LauncherSplitCrossfadePaneState {
        switch direction {
        case .detailToGuide:
            return LauncherSplitCrossfadePaneState(
                guide: LauncherOverlayHitTestState(passesHitTests: false, isHidden: false, alphaVisible: false),
                detail: LauncherOverlayHitTestState(passesHitTests: false, isHidden: false, alphaVisible: true)
            )
        case .guideToDetail:
            return LauncherSplitCrossfadePaneState(
                guide: LauncherOverlayHitTestState(passesHitTests: false, isHidden: false, alphaVisible: true),
                detail: LauncherOverlayHitTestState(passesHitTests: false, isHidden: false, alphaVisible: false)
            )
        }
    }

    public static func afterAnimation(_ direction: LauncherSplitCrossfadeDirection) -> LauncherSplitCrossfadePaneState {
        switch direction {
        case .detailToGuide:
            return LauncherSplitCrossfadePaneState(
                guide: LauncherOverlayHitTestState(passesHitTests: true, isHidden: false, alphaVisible: true),
                detail: LauncherOverlayHitTestState(passesHitTests: false, isHidden: true, alphaVisible: false)
            )
        case .guideToDetail:
            return LauncherSplitCrossfadePaneState(
                guide: LauncherOverlayHitTestState(passesHitTests: true, isHidden: true, alphaVisible: true),
                detail: LauncherOverlayHitTestState(passesHitTests: true, isHidden: false, alphaVisible: true)
            )
        }
    }
}
