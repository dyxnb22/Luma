import LumaCore

public enum BuiltInModules {
    /// Active modules registered at launch and warmed up by `ModuleHost`.
    public static func makeAll() -> [any LumaModule] {
        [
            AppsModule(),
            ClipboardModule(),
            CommandsModule(),
            TranslateModule()
        ]
    }

    /// Deferred modules kept in source but excluded from active dashboard, warmup, and default registration.
    public static func makeDeferred() -> [any LumaModule] {
        [
            WindowsModule(),
            CalculatorModule()
        ]
    }

    public static let accessibilityDependentModuleIDs: Set<ModuleIdentifier> = [.windows]

    public static func activeModulesRequireAccessibility() -> Bool {
        !accessibilityDependentModuleIDs.isDisjoint(with: Set(makeAll().map { type(of: $0).manifest.identifier }))
    }
}
