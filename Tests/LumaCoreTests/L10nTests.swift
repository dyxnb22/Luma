import Foundation
import Testing
import LumaCore
import LumaModules

@Test func l10nEnglishHomeSection() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .en
    #expect(LauncherHomeSectionKind.setup.title == "GET STARTED")
}

@Test func l10nChineseHomeSection() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .zhHans
    #expect(LauncherHomeSectionKind.setup.title == "开始使用")
}

@Test func searchEmptyStateLocalized() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .en
    let registry = BuiltInCommandRegistry.make()
    let message = SearchEmptyState.message(for: .empty, query: "", registry: registry)
    #expect(message.contains("Type to search"))
}
