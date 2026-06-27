import Foundation

public struct MemoryBudget: Sendable, Hashable {
    public var maxVisibleSearchResults: Int
    public var maxClipboardPreviewBytes: Int
    public var appIconCacheLimit: Int

    public init(
        maxVisibleSearchResults: Int = 5,
        maxClipboardPreviewBytes: Int = 100 * 1024,
        appIconCacheLimit: Int = 96
    ) {
        self.maxVisibleSearchResults = maxVisibleSearchResults
        self.maxClipboardPreviewBytes = maxClipboardPreviewBytes
        self.appIconCacheLimit = appIconCacheLimit
    }
}

public actor MemoryCoordinator {
    private(set) public var budget: MemoryBudget
    private var pressureEvents = 0

    public init(budget: MemoryBudget = MemoryBudget()) {
        self.budget = budget
    }

    public func notePressureEvent() {
        pressureEvents += 1
        budget.maxVisibleSearchResults = max(3, budget.maxVisibleSearchResults - 1)
        budget.appIconCacheLimit = max(32, budget.appIconCacheLimit / 2)
    }

    public func pressureEventCount() -> Int {
        pressureEvents
    }
}
