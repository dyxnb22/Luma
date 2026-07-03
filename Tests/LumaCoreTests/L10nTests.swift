import Foundation
import Testing
import LumaCore
import LumaModules

@Test func l10nEnglishHomeSection() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .en
    #expect(LauncherHomeSectionKind.openApps.title == "OPEN APPS")
}

@Test func l10nChineseHomeSection() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .zhHans
    #expect(LauncherHomeSectionKind.openApps.title == "打开应用")
}

@Test func searchEmptyStateLocalized() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .en
    let registry = BuiltInCommandRegistry.make()
    let message = SearchEmptyState.message(for: .empty, query: "", registry: registry)
    #expect(message.contains("Type to search"))
}

@Test func l10nDetailPlaceholderFormatsModuleTitle() {
    let previous = LumaLocale.choice
    defer { LumaLocale.choice = previous }
    LumaLocale.choice = .en
    let text = L10n.tr("translate.detail.placeholder", "Translate")
    #expect(text == "In Translate — Esc to go back")
    #expect(!text.contains("LocalizationValue"))
}
