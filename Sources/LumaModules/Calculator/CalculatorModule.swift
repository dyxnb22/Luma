import Foundation
import LumaCore

public actor CalculatorModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .calculator,
        displayName: "Calculator",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(20)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard query.normalized.rangeOfCharacter(from: CharacterSet(charactersIn: "0123456789+-*/().")) != nil else {
            return ModuleResult(items: [])
        }
        return ModuleResult(items: [])
    }
}
