import Foundation
import Testing
import LumaCore
import LumaModules

@Test func moduleSearchHintsPlaceholderComesFromRegistry() {
    #expect(ModuleSearchHints.placeholder(for: "rec") == "Log a book, movie, show, anime, or game")
    #expect(ModuleSearchHints.placeholder(for: "win left") == "Move focused window: left, right, max, center")
}

@Test func moduleSearchHintsUnknownFallsBackToDefault() {
    #expect(ModuleSearchHints.placeholder(for: "chrome") == CommandRegistry.defaultPlaceholder)
    #expect(ModuleSearchHints.placeholder(for: "recording") == CommandRegistry.defaultPlaceholder)
    #expect(ModuleSearchHints.placeholder(for: "wordle") == CommandRegistry.defaultPlaceholder)
}
