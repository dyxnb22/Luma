import Foundation
import LumaCore

@MainActor
final class LauncherSnapshotApplyPipeline {
    private let contentCoordinator: LauncherContentCoordinator
    private let isPanelActive: () -> Bool
    private let isQueryEmpty: () -> Bool
    private let onApplied: () -> Void

    private lazy var coalescer = LauncherSnapshotApplyCoalescer { [weak self] snapshot in
        self?.apply(snapshot: snapshot)
    }

    init(
        contentCoordinator: LauncherContentCoordinator,
        isPanelActive: @escaping () -> Bool,
        isQueryEmpty: @escaping () -> Bool,
        onApplied: @escaping () -> Void = {}
    ) {
        self.contentCoordinator = contentCoordinator
        self.isPanelActive = isPanelActive
        self.isQueryEmpty = isQueryEmpty
        self.onApplied = onApplied
    }

    func enqueue(_ snapshot: ResultSnapshot) {
        coalescer.enqueue(snapshot)
    }

    func cancelPending() {
        coalescer.cancel()
    }

    func apply(snapshot: ResultSnapshot) {
        let policy = LauncherSnapshotApplyPolicy.decision(
            isPanelActive: isPanelActive(),
            isQueryEmpty: isQueryEmpty()
        )
        guard policy.apply else {
            if policy.recordDroppedCounter {
                LauncherPerfCounters.increment(.snapshotApplyDropped)
            }
            return
        }
        LauncherPerfCounters.increment(.snapshotApply)
        contentCoordinator.apply(snapshot: snapshot)
        onApplied()
    }
}
