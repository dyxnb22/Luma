import Foundation
import Testing
import LumaServices

@Test func translationShortcutUnavailableThrows() async throws {
    let service = SystemTranslationService()
    do {
        _ = try await service.translateWithShortcut("hello", shortcutName: "Nonexistent Luma Translate Shortcut")
        Issue.record("Expected shortcutUnavailable error")
    } catch let error as SystemTranslationError {
        #expect(error == .shortcutUnavailable)
    }
}
