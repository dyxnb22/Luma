import Testing
@testable import LumaModules
@testable import LumaCore

@Test func featureCatalogContainsRequestedCards() {
    let titles = Set(FeatureCatalog.defaultCards().map(\.title))
    #expect(titles.contains("Translate"))
    #expect(titles.contains("Clipboard"))
    #expect(titles.contains("Secrets"))
    #expect(titles.contains("Layouts"))
    #expect(titles.contains("Notes"))
    #expect(titles.contains("Wordbook"))
}

@Test func featureCatalogDefaultPositionsAreUnique() {
    let positions = FeatureCatalog.defaultCards().map(\.position)
    #expect(Set(positions).count == positions.count)
}

@Test func dashboardCoreCardsIncludeNotesTodoWordbookSnippetsSecrets() {
    let cards = FeatureCatalog.dashboardCoreCards()
    #expect(cards.count == 7)
    let ids = Set(cards.map(\.id))
    #expect(ids.contains(.translate))
    #expect(ids.contains(.clipboard))
    #expect(ids.contains(.notes))
    #expect(ids.contains(.todo))
    #expect(ids.contains(.wordbook))
    #expect(ids.contains(.snippets))
    #expect(ids.contains(.secrets))
    #expect(cards.first { $0.id == .secrets }?.triggerKeyword == "secret ")
    #expect(cards.first { $0.id == .snippets }?.triggerKeyword == "s ")
}

@Test func dashboardCoreCardsDoNotExceedRouteBCeiling() {
    #expect(FeatureCatalog.dashboardCoreCards().count <= 8)
}

@Test func dashboardCoreCardsHaveWidgetStyleAndTrailingSpaceTriggers() {
    let cards = FeatureCatalog.dashboardCoreCards()
    for card in cards {
        #expect(card.widgetStyle != nil, "\(card.title) is missing widgetStyle")
        #expect(card.triggerKeyword.hasSuffix(" "), "\(card.title) trigger \"\(card.triggerKeyword)\" should end with a space")
    }
}

@Test func dashboardCoreCardsGradientHexStringsAreSixDigits() {
    let cards = FeatureCatalog.dashboardCoreCards()
    for card in cards {
        guard let style = card.widgetStyle else { continue }
        #expect(style.topHex.hasPrefix("#") && style.topHex.count == 7, "\(card.title) topHex malformed")
        #expect(style.bottomHex.hasPrefix("#") && style.bottomHex.count == 7, "\(card.title) bottomHex malformed")
    }
}
