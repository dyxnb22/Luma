import Testing
@testable import LumaCore

@Test func memoryCoordinatorReducesCacheUnderPressure() async {
    let coordinator = MemoryCoordinator(budget: MemoryBudget(maxVisibleSearchResults: 5, appIconCacheLimit: 96))
    await coordinator.notePressureEvent()
    let budget = await coordinator.budget
    #expect(budget.maxVisibleSearchResults == 4)
    #expect(budget.appIconCacheLimit == 48)
    #expect(await coordinator.pressureEventCount() == 1)
}

@Test func memoryCoordinatorKeepsMinimums() async {
    let coordinator = MemoryCoordinator(budget: MemoryBudget(maxVisibleSearchResults: 3, appIconCacheLimit: 32))
    await coordinator.notePressureEvent()
    let budget = await coordinator.budget
    #expect(budget.maxVisibleSearchResults == 3)
    #expect(budget.appIconCacheLimit == 32)
}
