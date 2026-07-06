import Foundation
import Testing
import LumaCore

@Test func snapshotApplyPolicyDropsInactivePanel() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: false, isQueryEmpty: false)
    #expect(decision.apply == false)
    #expect(decision.recordDroppedCounter)
}

@Test func snapshotApplyPolicyAllowsActiveSearch() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: true, isQueryEmpty: false)
    #expect(decision.apply)
    #expect(!decision.recordDroppedCounter)
}
