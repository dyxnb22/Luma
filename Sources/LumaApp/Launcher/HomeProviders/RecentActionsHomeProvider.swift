import Foundation
import LumaCore

struct RecentActionsHomeProvider: LauncherHomeProvider {
    private let recentItems: @Sendable () async -> [ResultItem]

    init(recentItems: @escaping @Sendable () async -> [ResultItem]) {
        self.recentItems = recentItems
    }

    func items() async -> [ResultItem] {
        Array(await recentItems().prefix(3))
    }
}
