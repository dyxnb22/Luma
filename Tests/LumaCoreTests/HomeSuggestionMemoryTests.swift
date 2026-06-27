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

@Test func homeSuggestionMemoryDeprioritizesCompletedCreateActions() async {
  let memory = HomeSuggestionMemory(completedCooldown: 120)
  #expect(await memory.isEligible(key: "contextual.clip-snippet", kind: .create))
  await memory.recordCompleted(key: "contextual.clip-snippet")
  #expect(await !memory.isEligible(key: "contextual.clip-snippet", kind: .create))
  #expect(await memory.isEligible(key: "contextual.daily", kind: .continueFlow))
}

@Test func homeSuggestionMemoryBoostsContinueFlowAndSessionContext() async {
  let memory = HomeSuggestionMemory()
  await memory.boostSessionContext(key: "contextual.current")
  let boosted = await memory.adjustedPriority(base: 80, key: "contextual.current", kind: .continueFlow)
  let plain = await memory.adjustedPriority(base: 80, key: "contextual.records", kind: .continueFlow)
  #expect(boosted > plain)
}

@Test func homeSuggestionMemoryRecordsCompletedOnDailyOpen() async {
  let memory = HomeSuggestionMemory(completedCooldown: 120)
  await memory.recordDailyNoteOpened()
  #expect(await memory.shouldSuppressDailyNoteSuggestion())
  #expect(await memory.shouldDeprioritizeCompleted(key: "contextual.daily"))
}
