import Testing
@testable import LumaModules

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
