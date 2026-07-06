import Foundation
import LumaCore

public actor ConfigurationStore: ConfigurationClient {
    private let defaults: UserDefaults
    private let enabledModulesKey = "enabledModules"
    private let clipboardMaxEntriesKey = "clipboardMaxEntries"
    private let clipboardMaxAgeDaysKey = "clipboardMaxAgeDays"
    private let clipboardMaxEntrySizeKBKey = "clipboardMaxEntrySizeKB"
    private let clipboardHistoryEnabledKey = "clipboardHistoryEnabled"
    private let clipboardIgnoredBundleIDsKey = "clipboardIgnoredBundleIDs"
    private let clipboardPasteBehaviorKey = "clipboardPasteBehavior"
    private let translationTargetLanguageKey = "translationTargetLanguage"
    private let secretsAutoClearSecondsKey = "secretsAutoClearSeconds"
    private let secretsRelockTimeoutSecondsKey = "secretsRelockTimeoutSeconds"
    private let secretsRequireUnlockOnLaunchKey = "secretsRequireUnlockOnLaunch"
    private let launcherLastModuleIDKey = "launcherLastModuleID"
    private let launcherLastQueryKey = "launcherLastQuery"
    private let launcherTranslateSourceTextKey = "launcherTranslateSourceText"
    private let launcherTranslateOutputTextKey = "launcherTranslateOutputText"
    private let latencyHUDEnabledKey = "latencyHUDEnabled"
    private let pinnedModuleIDsKey = "pinnedModuleIDs"
    private let warmupPolicyKey = "warmupPolicy"

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func enabledModules() async -> Set<ModuleIdentifier>? {
        guard let raw = defaults.stringArray(forKey: enabledModulesKey) else { return nil }
        return Set(raw.map(ModuleIdentifier.init(rawValue:)))
    }

    public func setEnabledModules(_ ids: Set<ModuleIdentifier>) {
        defaults.set(ids.map(\.rawValue).sorted(), forKey: enabledModulesKey)
    }

    public func clipboardMaxEntries() -> Int {
        let value = defaults.integer(forKey: clipboardMaxEntriesKey)
        return value > 0 ? value : 500
    }

    public func setClipboardMaxEntries(_ value: Int) {
        defaults.set(value, forKey: clipboardMaxEntriesKey)
    }

    public func clipboardMaxAgeDays() -> Int {
        let value = defaults.integer(forKey: clipboardMaxAgeDaysKey)
        return value > 0 ? value : 7
    }

    public func setClipboardMaxAgeDays(_ value: Int) {
        defaults.set(value, forKey: clipboardMaxAgeDaysKey)
    }

    public func clipboardMaxEntrySizeKB() -> Int {
        let value = defaults.integer(forKey: clipboardMaxEntrySizeKBKey)
        return value > 0 ? value : 100
    }

    public func setClipboardMaxEntrySizeKB(_ value: Int) {
        defaults.set(value, forKey: clipboardMaxEntrySizeKBKey)
    }

    public func clipboardHistoryEnabled() -> Bool {
        if defaults.object(forKey: clipboardHistoryEnabledKey) == nil { return true }
        return defaults.bool(forKey: clipboardHistoryEnabledKey)
    }

    public func setClipboardHistoryEnabled(_ value: Bool) {
        defaults.set(value, forKey: clipboardHistoryEnabledKey)
    }

    public func clipboardIgnoredBundleIDs() -> [String] {
        defaults.stringArray(forKey: clipboardIgnoredBundleIDsKey) ?? []
    }

    public func setClipboardIgnoredBundleIDs(_ value: [String]) {
        defaults.set(value, forKey: clipboardIgnoredBundleIDsKey)
    }

    public func clipboardPasteBehavior() -> String {
        defaults.string(forKey: clipboardPasteBehaviorKey) ?? "pasteDirectly"
    }

    public func setClipboardPasteBehavior(_ value: String) {
        defaults.set(value, forKey: clipboardPasteBehaviorKey)
    }

    public func translationTargetLanguage() -> String {
        defaults.string(forKey: translationTargetLanguageKey) ?? "en"
    }

    public func setTranslationTargetLanguage(_ value: String) {
        defaults.set(value, forKey: translationTargetLanguageKey)
    }

    public func secretsAutoClearSeconds() -> Int {
        let value = defaults.integer(forKey: secretsAutoClearSecondsKey)
        return value > 0 ? value : 10
    }

    public func setSecretsAutoClearSeconds(_ value: Int) {
        defaults.set(value, forKey: secretsAutoClearSecondsKey)
    }

    public func secretsRelockTimeoutSeconds() -> Int {
        let value = defaults.integer(forKey: secretsRelockTimeoutSecondsKey)
        return value > 0 ? value : 300
    }

    public func setSecretsRelockTimeoutSeconds(_ value: Int) {
        defaults.set(value, forKey: secretsRelockTimeoutSecondsKey)
    }

    public func secretsRequireUnlockOnLaunch() -> Bool {
        if defaults.object(forKey: secretsRequireUnlockOnLaunchKey) == nil { return true }
        return defaults.bool(forKey: secretsRequireUnlockOnLaunchKey)
    }

    public func setSecretsRequireUnlockOnLaunch(_ value: Bool) {
        defaults.set(value, forKey: secretsRequireUnlockOnLaunchKey)
    }

    public func launcherLastModuleID() -> String? {
        defaults.string(forKey: launcherLastModuleIDKey)
    }

    public func setLauncherLastModuleID(_ value: String?) {
        if let value {
            defaults.set(value, forKey: launcherLastModuleIDKey)
        } else {
            defaults.removeObject(forKey: launcherLastModuleIDKey)
        }
    }

    public func launcherLastQuery() -> String {
        defaults.string(forKey: launcherLastQueryKey) ?? ""
    }

    public func setLauncherLastQuery(_ value: String) {
        defaults.set(value, forKey: launcherLastQueryKey)
    }

    public func launcherTranslateSourceText() -> String {
        defaults.string(forKey: launcherTranslateSourceTextKey) ?? ""
    }

    public func setLauncherTranslateSourceText(_ value: String) {
        defaults.set(value, forKey: launcherTranslateSourceTextKey)
    }

    public func launcherTranslateOutputText() -> String {
        defaults.string(forKey: launcherTranslateOutputTextKey) ?? ""
    }

    public func setLauncherTranslateOutputText(_ value: String) {
        defaults.set(value, forKey: launcherTranslateOutputTextKey)
    }

    public func latencyHUDEnabled() -> Bool {
        if defaults.object(forKey: latencyHUDEnabledKey) == nil { return false }
        return defaults.bool(forKey: latencyHUDEnabledKey)
    }

    public func setLatencyHUDEnabled(_ value: Bool) {
        defaults.set(value, forKey: latencyHUDEnabledKey)
    }

    public func pinnedModuleIDs() async -> Set<ModuleIdentifier> {
        guard let raw = defaults.stringArray(forKey: pinnedModuleIDsKey) else {
            return ModuleWarmupDefaults.defaultPinnedModuleIDs
        }
        return Set(raw.map(ModuleIdentifier.init(rawValue:)))
    }

    public func setPinnedModuleIDs(_ ids: Set<ModuleIdentifier>) {
        defaults.set(ids.map(\.rawValue).sorted(), forKey: pinnedModuleIDsKey)
    }

    public func warmupPolicy() async -> WarmupPolicy {
        guard let raw = defaults.string(forKey: warmupPolicyKey),
              let policy = WarmupPolicy(rawValue: raw) else {
            return .eagerPinnedOnly
        }
        return policy
    }

    public func setWarmupPolicy(_ policy: WarmupPolicy) {
        defaults.set(policy.rawValue, forKey: warmupPolicyKey)
    }

    private static let schemaVersionKey = "configSchemaVersion"
    private static let currentSchemaVersion = 1

    /// Bumps `configSchemaVersion` when defaults change. Add migration steps here as schema evolves.
    public func migrateIfNeeded() async {
        let version = defaults.integer(forKey: Self.schemaVersionKey)
        guard version < Self.currentSchemaVersion else { return }
        defaults.set(Self.currentSchemaVersion, forKey: Self.schemaVersionKey)
    }
}
