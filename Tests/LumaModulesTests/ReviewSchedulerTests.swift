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

@Test func fuzzyBackwardCompatUsesKnownSchedule() {
    let result = ReviewScheduler.schedule(familiarity: .fuzzy, currentStage: 3, wrongCount: 0)
    #expect(result.stage == 4)
    #expect(result.delay == .seconds(24 * 60 * 60))
}

@Test func masteredCaseSchedulesFarFuture() {
    let result = ReviewScheduler.schedule(familiarity: .mastered, currentStage: 2, wrongCount: 0)
    #expect(result.stage == ReviewScheduler.intervals.count)
    #expect(result.delay >= .seconds(60 * 60 * 24 * 365 * 50))
}
