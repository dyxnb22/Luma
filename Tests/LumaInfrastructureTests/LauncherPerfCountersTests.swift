import Foundation
import Testing
import LumaCore

@Test func launcherPerfCountersTrackIncrements() {
    LauncherPerfCounters.reset()
    LauncherPerfCounters.increment(.sessionPersist)
    LauncherPerfCounters.increment(.sessionPersist)
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 2)
    LauncherPerfCounters.reset()
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 0)
}

@Test func launcherPerfCountersTrackNewStabilityKeys() {
    LauncherPerfCounters.reset()
    LauncherPerfCounters.increment(.queryCancelOnHide)
    LauncherPerfCounters.increment(.snapshotApplyDropped)
    LauncherPerfCounters.increment(.panelHide)
    LauncherPerfCounters.increment(.moduleWarmupStarted)
    LauncherPerfCounters.increment(.moduleWarmupFinished)
    LauncherPerfCounters.increment(.moduleHandleCold)
    LauncherPerfCounters.increment(.cacheQueryHit)
    LauncherPerfCounters.increment(.cacheQueryMiss)

    #expect(LauncherPerfCounters.count(for: .queryCancelOnHide) == 1)
    #expect(LauncherPerfCounters.count(for: .snapshotApplyDropped) == 1)
    #expect(LauncherPerfCounters.count(for: .panelHide) == 1)
    #expect(LauncherPerfCounters.count(for: .moduleWarmupStarted) == 1)
    #expect(LauncherPerfCounters.count(for: .moduleWarmupFinished) == 1)
    #expect(LauncherPerfCounters.count(for: .moduleHandleCold) == 1)
    #expect(LauncherPerfCounters.count(for: .cacheQueryHit) == 1)
    #expect(LauncherPerfCounters.count(for: .cacheQueryMiss) == 1)
}
