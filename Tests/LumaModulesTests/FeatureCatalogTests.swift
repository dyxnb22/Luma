import Testing
@testable import LumaModules
@testable import LumaCore

@Test func featureCatalogContainsRequestedCards() {
    let titles = Set(FeatureCatalog.moduleDetailMetadata().map(\.title))
    #expect(titles.contains("Translate"))
    #expect(titles.contains("Clipboard"))
    #expect(titles.contains("Secrets"))
    #expect(titles.contains("Notes"))
    #expect(titles.contains("Wordbook"))
}

@Test func featureCatalogDefaultPositionsAreUnique() {
    let positions = FeatureCatalog.moduleDetailMetadata().map(\.position)
    #expect(Set(positions).count == positions.count)
}

@Test func moduleDetailMetadataIncludesActiveModules() {
    let cards = FeatureCatalog.moduleDetailMetadata()
    #expect(cards.count == 8)
    let ids = Set(cards.map(\.id))
    #expect(ids.contains(.translate))
    #expect(ids.contains(.clipboard))
    #expect(ids.contains(.notes))
    #expect(ids.contains(.todo))
    #expect(ids.contains(.wordbook))
    #expect(ids.contains(.snippets))
    #expect(ids.contains(.secrets))
    #expect(ids.contains(.quicklinks))
    #expect(cards.first { $0.id == .secrets }?.triggerKeyword == "sec ")
    #expect(cards.first { $0.id == .snippets }?.triggerKeyword == "s ")
    #expect(cards.first { $0.id == .quicklinks }?.triggerKeyword == "ql ")
}

@Test func moduleDetailMetadataDoesNotExceedCeiling() {
    #expect(FeatureCatalog.moduleDetailMetadata().count <= 8)
}

@Test func moduleDetailMetadataHasWidgetStyleAndTrailingSpaceTriggers() {
    let cards = FeatureCatalog.moduleDetailMetadata()
    for card in cards {
        #expect(card.widgetStyle != nil, "\(card.title) is missing widgetStyle")
        #expect(card.triggerKeyword.hasSuffix(" "), "\(card.title) trigger \"\(card.triggerKeyword)\" should end with a space")
    }
}

@Test func moduleDetailMetadataGradientHexStringsAreSixDigits() {
    let cards = FeatureCatalog.moduleDetailMetadata()
    for card in cards {
        guard let style = card.widgetStyle else { continue }
        #expect(style.topHex.hasPrefix("#") && style.topHex.count == 7, "\(card.title) topHex malformed")
        #expect(style.bottomHex.hasPrefix("#") && style.bottomHex.count == 7, "\(card.title) bottomHex malformed")
    }
}
