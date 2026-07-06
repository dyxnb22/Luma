import Foundation
import LumaCore
import LumaInfrastructure
import Testing
@testable import LumaApp

@Test @MainActor func snapshotApplyCoalescerMergesBurstsWithinOneFrame() async {
    LauncherPerfCounters.reset()
    var applyCount = 0
    let coalescer = LauncherSnapshotApplyCoalescer { _ in
        applyCount += 1
    }

    for index in 0..<5 {
        coalescer.enqueue(ResultSnapshot(querySequence: UInt64(index), items: []))
    }
    let coalescedDuringBurst = LauncherPerfCounters.count(for: .snapshotApplyCoalesced)

    try? await Task.sleep(for: .milliseconds(25))
    #expect(applyCount == 1)
    #expect(coalescedDuringBurst >= 4)
}

@Test @MainActor func snapshotApplyCoalescerFlushNowBypassesDebounce() {
    var applied: [UInt64] = []
    let coalescer = LauncherSnapshotApplyCoalescer { snapshot in
        applied.append(snapshot.querySequence)
    }

    coalescer.enqueue(ResultSnapshot(querySequence: 9, items: []))
    coalescer.flushNow()

    #expect(applied == [9])
}
