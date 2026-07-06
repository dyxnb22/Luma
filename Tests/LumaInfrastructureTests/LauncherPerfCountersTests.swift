import Foundation
import Testing
@testable import LumaInfrastructure

@Test func launcherPerfCountersTrackIncrements() {
    LauncherPerfCounters.reset()
    LauncherPerfCounters.increment(.sessionPersist)
    LauncherPerfCounters.increment(.sessionPersist)
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 2)
    LauncherPerfCounters.reset()
    #expect(LauncherPerfCounters.count(for: .sessionPersist) == 0)
}
