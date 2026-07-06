import Foundation
import LumaCore
import Testing

@Test func cancellationGenerationBumpInvalidatesPrevious() {
    var generation = CancellationGeneration()
    let first = generation.bump()
    #expect(generation.isCurrent(first))
    _ = generation.bump()
    #expect(!generation.isCurrent(first))
}

@Test func cancellationGenerationStartsAtZero() {
    let generation = CancellationGeneration()
    #expect(generation.current == 0)
    #expect(!generation.isCurrent(1))
}
