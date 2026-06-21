import Testing
@testable import LumaModules

@Test func knownWordAdvancesToNextStage() {
    let result = ReviewScheduler.schedule(familiarity: .known, currentStage: 0, wrongCount: 0)
    #expect(result.stage == 1)
    #expect(result.delay == .seconds(5 * 60))
}

@Test func unknownWordReturnsToFirstStage() {
    let result = ReviewScheduler.schedule(familiarity: .unknown, currentStage: 5, wrongCount: 2)
    #expect(result.stage == 0)
    #expect(result.delay == .seconds(30 * 60))
}

@Test func fuzzyWordKeepsCurrentStageWithStageBasedDelay() {
    let result = ReviewScheduler.schedule(familiarity: .fuzzy, currentStage: 3, wrongCount: 0)
    #expect(result.stage == 3)
    #expect(result.delay == .seconds(24 * 60 * 60))
}
