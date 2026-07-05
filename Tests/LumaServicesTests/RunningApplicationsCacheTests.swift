import Foundation
import Testing
@testable import LumaServices

@Test func runningApplicationsCacheReturnsStaleImmediatelyWhenTTLExpired() async {
    let cache = RunningApplicationsCache(ttl: 0.001)
    await cache.seedCacheForTesting(["com.example.stale"], lastRefresh: .distantPast)

    let start = ContinuousClock.now
    let bundleIDs = await cache.runningBundleIDs()
    let elapsed = start.duration(to: .now)
    let elapsedMs = Double(elapsed.components.seconds) * 1000
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000

    #expect(bundleIDs == ["com.example.stale"])
    #expect(elapsedMs < 50)
}

@Test func runningApplicationsCacheCoalescesConcurrentStaleRefreshes() async {
    let cache = RunningApplicationsCache(ttl: 0.001)
    await cache.seedCacheForTesting(["com.example.one"], lastRefresh: .distantPast)

    async let first = cache.runningBundleIDs()
    async let second = cache.runningBundleIDs()
    _ = await (first, second)

    try? await Task.sleep(for: .milliseconds(100))
    let refreshCount = await cache.refreshCallCount
    #expect(refreshCount == 1)
}

@Test func runningApplicationsCacheStartStopObserversArePaired() async {
    let cache = RunningApplicationsCache()
    await cache.startMonitoring()
    #expect(await cache.hasObserversInstalled == true)

    await cache.stopMonitoring()
    #expect(await cache.hasObserversInstalled == false)
}
