import Foundation

/// Pure gating for launcher snapshot UI apply (hide stale-apply guard).
public enum LauncherSnapshotApplyPolicy {
    public struct Decision: Equatable, Sendable {
        public let apply: Bool
        public let recordDroppedCounter: Bool

        public init(apply: Bool, recordDroppedCounter: Bool) {
            self.apply = apply
            self.recordDroppedCounter = recordDroppedCounter
        }
    }

    public static func decision(isPanelActive: Bool, isQueryEmpty: Bool) -> Decision {
        if !isPanelActive {
            return Decision(apply: false, recordDroppedCounter: true)
        }
        if isQueryEmpty {
            return Decision(apply: false, recordDroppedCounter: false)
        }
        return Decision(apply: true, recordDroppedCounter: false)
    }
}
