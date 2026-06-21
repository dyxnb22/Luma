import Testing
@testable import LumaCore

@Test func fuzzyMatcherScoresPrefixAboveMiss() {
    let prefix = FuzzyMatcher.score(query: "saf", target: "safari")
    let miss = FuzzyMatcher.score(query: "xyz", target: "safari")
    #expect(prefix > miss)
}
