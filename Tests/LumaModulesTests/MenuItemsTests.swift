import Testing
@testable import LumaModules
import LumaServices

@Test func menuItemsIndexWeightsLeafTitleHighest() {
    let records = [
        MenuItemRecord(bundleID: "app", titlePath: ["View", "Fold", "Fold All Block Comments"], shortcutDisplay: nil, axPath: [0, 1], isEnabled: true),
        MenuItemRecord(bundleID: "app", titlePath: ["Fold All", "Other"], shortcutDisplay: nil, axPath: [1, 0], isEnabled: true)
    ]
    #expect(MenuItemsIndex.search(records, query: "fold all").first?.record.titlePath.last == "Fold All Block Comments")
}

@Test func menuItemsIndexReturnsEmptyForMiss() {
    let records = [
        MenuItemRecord(bundleID: "app", titlePath: ["File", "Save"], shortcutDisplay: "⌘S", axPath: [0, 0], isEnabled: true)
    ]
    #expect(MenuItemsIndex.search(records, query: "xyz").isEmpty)
}

@Test func menuItemsIndexLimitsResults() {
    let records = (0..<20).map {
        MenuItemRecord(bundleID: "app", titlePath: ["File", "Save \($0)"], shortcutDisplay: nil, axPath: [0, $0], isEnabled: true)
    }
    #expect(MenuItemsIndex.search(records, query: "save", limit: 8).count == 8)
}
