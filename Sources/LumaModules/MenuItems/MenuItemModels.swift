import Foundation
import LumaServices

public struct MenuItemsConfig: Codable, Sendable, Hashable {
    public var disabledBundleIDs: [String]
    public var pinnedItems: [String]

    public init(disabledBundleIDs: [String] = ["com.apple.finder"], pinnedItems: [String] = []) {
        self.disabledBundleIDs = disabledBundleIDs
        self.pinnedItems = pinnedItems
    }
}

public struct MenuItemMatch: Sendable, Hashable {
    public let record: MenuItemRecord
    public let score: Double
}
