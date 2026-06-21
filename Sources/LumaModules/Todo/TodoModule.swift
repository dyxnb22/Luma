import Foundation
import LumaCore

public actor TodoModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .todo,
        displayName: "Todo",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 2,
        queryTimeout: .milliseconds(30)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        ModuleResult(items: [])
    }
}
