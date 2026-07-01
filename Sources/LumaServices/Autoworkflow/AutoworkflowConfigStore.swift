import Foundation

/// Persists AutoworkflowConfig to UserDefaults. Separate from ConfigurationStore
/// to keep autoworkflow settings decoupled from core Luma settings.
public actor AutoworkflowConfigStore {
    private let defaults: UserDefaults

    private enum Key {
        static let autoworkflowPath = "aw_path"
        static let stateRoot = "aw_stateRoot"
        static let defaultPlanner = "aw_planner"
        static let defaultReviewer = "aw_reviewer"
        static let defaultImplementer = "aw_implementer"
        static let defaultModel = "aw_model"
    }

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func load() -> AutoworkflowConfig {
        AutoworkflowConfig(
            autoworkflowPath: defaults.string(forKey: Key.autoworkflowPath) ?? "\(NSHomeDirectory())/autoworkflow",
            stateRoot: defaults.string(forKey: Key.stateRoot) ?? NSHomeDirectory() + "/.cc-loop",
            defaultPlanner: defaults.string(forKey: Key.defaultPlanner) ?? "claude-code",
            defaultReviewer: defaults.string(forKey: Key.defaultReviewer) ?? "claude-code",
            defaultImplementer: defaults.string(forKey: Key.defaultImplementer) ?? "cursor",
            defaultModel: defaults.string(forKey: Key.defaultModel) ?? "sonnet"
        )
    }

    public func save(_ config: AutoworkflowConfig) {
        // Never persist empty path keys — clearing a field in Settings must not overwrite stored paths.
        let existing = load()
        let path = config.autoworkflowPath.trimmingCharacters(in: .whitespacesAndNewlines)
        let stateRoot = config.stateRoot.trimmingCharacters(in: .whitespacesAndNewlines)
        defaults.set(path.isEmpty ? existing.autoworkflowPath : path, forKey: Key.autoworkflowPath)
        defaults.set(stateRoot.isEmpty ? existing.stateRoot : stateRoot, forKey: Key.stateRoot)
        defaults.set(config.defaultPlanner, forKey: Key.defaultPlanner)
        defaults.set(config.defaultReviewer, forKey: Key.defaultReviewer)
        defaults.set(config.defaultImplementer, forKey: Key.defaultImplementer)
        defaults.set(config.defaultModel, forKey: Key.defaultModel)
    }

    public func save(key: String, value: String) {
        defaults.set(value, forKey: "aw_\(key)")
    }

    public func get(key: String) -> String? {
        defaults.string(forKey: "aw_\(key)")
    }
}
