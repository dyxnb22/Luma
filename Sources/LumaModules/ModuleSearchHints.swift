import Foundation
import LumaCore

/// Contextual search placeholder strings keyed by the command registry.
public enum ModuleSearchHints {
    public static let `default` = CommandRegistry.defaultPlaceholder
    public static let cheatSheet = "Search apps, paste, translate, todo…"

    private static let registry = BuiltInCommandRegistry.make()

    public static func placeholder(for query: String) -> String {
        registry.placeholder(for: query)
    }
}
