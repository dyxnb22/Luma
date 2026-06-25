import Testing
@testable import LumaModules

@Test func mediaParserCapturesFullLine() {
    let result = MediaParser.parse("Oppenheimer movie 9")
    #expect(result.mode == .capture(partial: false))
    #expect(result.title == "Oppenheimer")
    #expect(result.category == .movie)
    #expect(result.rating == 9)
    #expect(result.status == .done)
    #expect(result.tags.isEmpty)
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
    #expect(result.rating == nil)
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

@Test func mediaParserCapturesChineseBookWithTags() {
    let result = MediaParser.parse("三体 book done 9 #sci-fi #favorite")
    #expect(result.title == "三体")
    #expect(result.category == .book)
    #expect(result.status == .done)
    #expect(result.rating == 9)
    #expect(result.tags == ["sci-fi", "favorite"])
}

@Test func mediaParserCapturesChineseAnimeWatching() {
    let result = MediaParser.parse("葬送的芙莉莲 anime watching")
    #expect(result.title == "葬送的芙莉莲")
    #expect(result.category == .anime)
    #expect(result.status == .inProgress)
}

@Test func mediaParserCapturesChineseGameDone() {
    let result = MediaParser.parse("黑神话 game 通关 8")
    #expect(result.title == "黑神话")
    #expect(result.category == .game)
    #expect(result.status == .done)
    #expect(result.rating == 8)
}

@Test func mediaParserCapturesTVWithTag() {
    let result = MediaParser.parse("The Bear tv #food")
    #expect(result.title == "The Bear")
    #expect(result.category == .tv)
    #expect(result.tags == ["food"])
    #expect(result.status == .done)
}

@Test func mediaParserRatingAliases() {
    #expect(MediaParser.parse("Dune movie ★8").rating == 8)
    #expect(MediaParser.parse("Dune movie rating:8").rating == 8)
    #expect(MediaParser.parse("Dune movie r8").rating == 8)
    #expect(MediaParser.parse("Dune movie 8/10").rating == 8)
}

@Test func mediaParserChineseCategoryAliases() {
    #expect(MediaParser.parse("三体 书 done").category == .book)
    #expect(MediaParser.parse("沙丘 电影 8").category == .movie)
    #expect(MediaParser.parse("Bear 剧 watching").category == .tv)
    #expect(MediaParser.parse("芙莉莲 番 watching").category == .anime)
    #expect(MediaParser.parse("黑神话 游戏 通关").category == .game)
}

@Test func mediaParserChineseStatusAliases() {
    #expect(MediaParser.parse("三体 book 想看").status == .planned)
    #expect(MediaParser.parse("三体 book 在读").status == .inProgress)
    #expect(MediaParser.parse("三体 book 读完").status == .done)
    #expect(MediaParser.parse("黑神话 game 弃坑").status == .abandoned)
}

@Test func mediaParserExtractsTagsFromMiddle() {
    let result = MediaParser.parse("三体 #sci-fi book done 9")
    #expect(result.title == "三体")
    #expect(result.tags == ["sci-fi"])
    #expect(result.category == .book)
}

@Test func mediaParserRemovesTagsInSearchMode() {
    let result = MediaParser.parse("Dune #sci-fi")
    #expect(result.mode == .search)
    #expect(result.title == "Dune")
    #expect(result.tags == ["sci-fi"])
}
