import Testing
@testable import LumaModules

@Test func mediaParserCapturesFullLine() {
    let result = MediaParser.parse("Oppenheimer movie 9")
    #expect(result.mode == .capture(partial: false))
    #expect(result.title == "Oppenheimer")
    #expect(result.category == .movie)
    #expect(result.rating == 9)
    #expect(result.status == .done)
}

@Test func mediaParserCapturesBookWithRating() {
    let result = MediaParser.parse("The Three-Body Problem book 10")
    #expect(result.title == "The Three-Body Problem")
    #expect(result.category == .book)
    #expect(result.rating == 10)
}

@Test func mediaParserCapturesDroppedGame() {
    let result = MediaParser.parse("Cyberpunk 2077 game dropped 4")
    #expect(result.title == "Cyberpunk 2077")
    #expect(result.category == .game)
    #expect(result.rating == 4)
    #expect(result.status == .abandoned)
}

@Test func mediaParserCapturesWatchingAnime() {
    let result = MediaParser.parse("Frieren anime watching")
    #expect(result.title == "Frieren")
    #expect(result.category == .anime)
    #expect(result.status == .inProgress)
    #expect(result.rating == nil)
}

@Test func mediaParserKeeps1984AsTitle() {
    let result = MediaParser.parse("1984 book")
    #expect(result.title == "1984")
    #expect(result.category == .book)
}

@Test func mediaParserSearchModeForPlainQuery() {
    let result = MediaParser.parse("oppen")
    #expect(result.mode == .search)
    #expect(result.title == "oppen")
    #expect(result.hadDSLToken == false)
}

@Test func mediaParserPartialCaptureWithoutCategory() {
    let result = MediaParser.parse("The Bear")
    #expect(result.mode == .search)
}

@Test func mediaParserRatingWithoutCategoryIsPartialCapture() {
    let result = MediaParser.parse("foo 9")
    #expect(result.mode == .capture(partial: true))
    #expect(result.title == "foo")
    #expect(result.rating == 9)
}
