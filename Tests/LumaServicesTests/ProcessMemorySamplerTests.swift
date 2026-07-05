import Foundation
import LumaCore
import Testing
@testable import LumaServices

@Test func processMemorySamplerReturnsCachedSnapshotImmediately() async {
    let sampler = ProcessMemorySampler(sampleTTL: 60)
    let samples = [
        RunningApplicationMemory(bundleID: "com.apple.Safari", name: "Safari", residentBytes: 512 * 1024 * 1024)
    ]
    await sampler.seedForTesting(samples)

    let start = ContinuousClock.now
    let top = await sampler.topApplications(limit: 8)
    let elapsed = start.duration(to: .now)
    let elapsedMs = Double(elapsed.components.seconds) * 1000
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000

    #expect(top.count == 1)
    #expect(top[0].name == "Safari")
    #expect(elapsedMs < 20)
    #expect(await sampler.psInvocationCount == 0)
}

@Test func processMemoryServiceReadsFromSamplerWithoutSpawningPS() async {
    let sampler = ProcessMemorySampler(sampleTTL: 60)
    await sampler.seedForTesting([
        RunningApplicationMemory(bundleID: "com.apple.finder", name: "Finder", residentBytes: 256 * 1024 * 1024)
    ])
    let service = ProcessMemoryService(sampler: sampler)

    let top = await service.topApplications(limit: 4)
    #expect(top.first?.name == "Finder")
    #expect(await sampler.psInvocationCount == 0)
}
