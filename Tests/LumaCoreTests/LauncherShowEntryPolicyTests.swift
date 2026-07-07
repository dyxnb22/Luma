import Foundation
import LumaCore
import Testing

@Test func carbonHotkeyDoesNotShowWhenAlreadyVisible() {
    #expect(!LauncherShowEntryPolicy.shouldBeginShowWhenAlreadyVisible(reason: .carbonHotkey))
    #expect(LauncherShowEntryPolicy.appliesCarbonShowDebounce(reason: .carbonHotkey))
}

@Test func menuBarMayShowWhenAlreadyVisible() {
    #expect(LauncherShowEntryPolicy.shouldBeginShowWhenAlreadyVisible(reason: .menuBar))
    #expect(!LauncherShowEntryPolicy.appliesCarbonShowDebounce(reason: .menuBar))
}

@Test func restoreAndQAAllowVisibleReshow() {
    #expect(LauncherShowEntryPolicy.shouldBeginShowWhenAlreadyVisible(reason: .restore))
    #expect(LauncherShowEntryPolicy.shouldBeginShowWhenAlreadyVisible(reason: .qa))
}
