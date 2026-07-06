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

@Test @MainActor func cancelLauncherAsyncWorkCancelsWorkbenchPreviewRegistryEntry() async {
    let registry = LauncherTaskRegistry()
    var finished = false
    let preview = Task {
        do {
            try await Task.sleep(for: .seconds(60))
        } catch {
            return
        }
        finished = true
    }
    registry.register(key: "workbenchPreview", task: preview)
    registry.cancelAll()
    try? await Task.sleep(for: .milliseconds(50))
    #expect(finished == false)
    #expect(registry.contains(key: "workbenchPreview") == false)
}

@Test @MainActor func workbenchPreviewPathDropsApplyAfterAsyncCancel() async {
    LauncherPerfCounters.reset()
    var applied = 0
    var panelActive = true
    let pipeline = LauncherSnapshotApplyPipeline(
        contentCoordinator: LauncherContentCoordinator(
            listView: LauncherListView(),
            detailContainer: LauncherOverlayHostView(),
            detailTopBar: NSView(),
            detailTitleLabel: NSTextField(labelWithString: ""),
            contentContainer: NSView()
        ),
        isPanelActive: { panelActive },
        isQueryEmpty: { false },
        onApplied: { applied += 1 }
    )
    let snapshot = ResultSnapshot(
        querySequence: 7,
        items: [
            ResultItem(
                id: ResultID(module: .workbench, key: "preview"),
                title: "Preview",
                titleAttributed: AttributedString("Preview"),
                icon: .symbol("hammer"),
                primaryAction: Action(
                    id: ActionID(module: .workbench, key: "preview"),
                    title: "Preview",
                    kind: .noop
                ),
                rankingHints: RankingHints()
            )
        ]
    )
    pipeline.enqueue(snapshot)
    panelActive = false
    pipeline.cancelPending()
    pipeline.apply(snapshot: snapshot)
    #expect(applied == 0)
    #expect(LauncherPerfCounters.count(for: .snapshotApplyDropped) == 1)
}

import AppKit
