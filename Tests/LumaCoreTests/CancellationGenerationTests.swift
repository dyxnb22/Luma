import LumaCore
import Testing

@Test func cancellationGenerationBumpInvalidatesPriorToken() {
    var generation = CancellationGeneration()
    let captured = generation.current
    generation.bump()
    #expect(!generation.isCurrent(captured))
    #expect(generation.isCurrent(generation.current))
}

@Test func cancellationGenerationTracksMonotonicBumps() {
    var generation = CancellationGeneration()
    #expect(generation.current == 0)
    #expect(generation.bump() == 1)
    #expect(generation.bump() == 2)
    #expect(generation.isCurrent(2))
    #expect(!generation.isCurrent(1))
}

@Test func cancelPendingRestoreScenarioDiscardsStaleApply() {
    var restoreGeneration = CancellationGeneration()
    let capturedForRestore = restoreGeneration.current
    restoreGeneration.bump()
    #expect(!restoreGeneration.isCurrent(capturedForRestore))
}
