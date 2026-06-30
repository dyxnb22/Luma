import LumaCore

/// Built-in command registry aggregated from module bundles.
public enum BuiltInCommandRegistry {
    public static func make() -> CommandRegistry {
        ModuleRegistry.makeCommandRegistry()
    }
}
