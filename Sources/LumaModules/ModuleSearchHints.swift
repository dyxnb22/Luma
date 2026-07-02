import Foundation
import LumaCore

/// Contextual search placeholder strings keyed by the command registry.
public enum ModuleSearchHints {
    public static var `default`: String { CommandRegistry.defaultPlaceholder }
    public static var cheatSheet: String { L10n.tr("search.placeholder.cheatSheet") }

    private static let registry = BuiltInCommandRegistry.make()

    public static func placeholder(for query: String) -> String {
        registry.placeholder(for: query)
    }
}
