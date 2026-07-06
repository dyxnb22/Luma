import Foundation
import LumaCore
import LumaInfrastructure
import Testing
@testable import LumaApp

@Test func launcherSnapshotApplyPolicyDropsWhenPanelInactive() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: false, isQueryEmpty: false)
    #expect(decision == LauncherSnapshotApplyPolicy.Decision(apply: false, recordDroppedCounter: true))
}

@Test func launcherSnapshotApplyPolicySkipsEmptyQueryWithoutDroppedCounter() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: true, isQueryEmpty: true)
    #expect(decision == LauncherSnapshotApplyPolicy.Decision(apply: false, recordDroppedCounter: false))
}

@Test func launcherSnapshotApplyPolicyAllowsActiveNonemptyQuery() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: true, isQueryEmpty: false)
    #expect(decision == LauncherSnapshotApplyPolicy.Decision(apply: true, recordDroppedCounter: false))
}

@Test @MainActor func cancelActiveQueryAndSnapshotApplyIncrementsDroppedCounter() async {
    LauncherPerfCounters.reset()
    var droppedApplies = 0
    var applied = 0

    let coalescer = LauncherSnapshotApplyCoalescer { _ in
        let policy = LauncherSnapshotApplyPolicy.decision(
            isPanelActive: false,
            isQueryEmpty: false
        )
        guard policy.apply else {
            if policy.recordDroppedCounter {
                LauncherPerfCounters.increment(.snapshotApplyDropped)
                droppedApplies += 1
            }
            return
        }
        LauncherPerfCounters.increment(.snapshotApply)
        applied += 1
    }

    coalescer.enqueue(ResultSnapshot(querySequence: 1, items: []))
    coalescer.flushNow()

    #expect(applied == 0)
    #expect(droppedApplies == 1)
    #expect(LauncherPerfCounters.count(for: .snapshotApplyDropped) == 1)
    #expect(LauncherPerfCounters.count(for: .snapshotApply) == 0)
}

@Test func launcherWindowControllerHideCancelsActiveQuery() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherWindowController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("cancelActiveQueryAndSnapshotApply()"))
    #expect(source.contains("cancelPendingRestore()"))
}

@Test func launcherRootControllerUsesSnapshotApplyPolicy() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("LauncherSnapshotApplyPolicy.decision"))
    #expect(source.contains("cancelActiveQueryAndSnapshotApply()"))
}
