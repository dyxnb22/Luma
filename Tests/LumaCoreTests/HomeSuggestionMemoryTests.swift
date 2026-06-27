import Foundation
import Testing
import LumaCore

@Test func homeSuggestionMemorySuppressesRecentSuggestions() async {
    let memory = HomeSuggestionMemory(repeatCooldown: 60, dailyNoteCooldown: 120)
    await memory.recordShown(keys: ["contextual.daily"])
    #expect(await memory.shouldSuppressSuggestion(key: "contextual.daily"))
    #expect(await !memory.shouldSuppressSuggestion(key: "contextual.records"))
}

@Test func homeSuggestionMemoryDeprioritizesDailyNoteAfterOpen() async {
    let memory = HomeSuggestionMemory(repeatCooldown: 60, dailyNoteCooldown: 120)
    #expect(await !memory.shouldSuppressDailyNoteSuggestion())
    await memory.recordDailyNoteOpened()
    #expect(await memory.shouldSuppressDailyNoteSuggestion())
}

@Test func homeSuggestionMemoryPrunesStaleEntries() async {
    let memory = HomeSuggestionMemory(repeatCooldown: 0.01, dailyNoteCooldown: 0.01)
    await memory.recordShown(keys: ["contextual.clip-note"])
    try? await Task.sleep(for: .milliseconds(30))
    #expect(await !memory.shouldSuppressSuggestion(key: "contextual.clip-note"))
}
