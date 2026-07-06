import Foundation
import LumaCore
import Testing
@testable import LumaApp

@Test func workbenchPreviewSnapshotUsesPanelActivePolicy() {
    let decision = LauncherSnapshotApplyPolicy.decision(isPanelActive: false, isQueryEmpty: false)
    #expect(decision.apply == false)
    #expect(decision.recordDroppedCounter == true)
}

@Test @MainActor func workbenchPreviewPolicyDropsWhenPanelInactive() async {
    LauncherPerfCounters.reset()
    var applied = 0
    let pipeline = LauncherSnapshotApplyPipeline(
        contentCoordinator: LauncherContentCoordinator(
            listView: LauncherListView(),
            detailContainer: LauncherOverlayHostView(),
            detailTopBar: NSView(),
            detailTitleLabel: NSTextField(labelWithString: ""),
            contentContainer: NSView()
        ),
        isPanelActive: { false },
        isQueryEmpty: { false },
        onApplied: { applied += 1 }
    )
    pipeline.apply(snapshot: ResultSnapshot(querySequence: 0, items: []))
    #expect(applied == 0)
    #expect(LauncherPerfCounters.count(for: .snapshotApplyDropped) == 1)
}

import AppKit
