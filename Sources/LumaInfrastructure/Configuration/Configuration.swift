import Foundation
import LumaCore

public actor ConfigurationStore: ConfigurationClient {
    private let defaults: UserDefaults
    private let enabledModulesKey = "enabledModules"
    private let clipboardMaxEntriesKey = "clipboardMaxEntries"
    private let clipboardMaxAgeDaysKey = "clipboardMaxAgeDays"
    private let clipboardMaxEntrySizeKBKey = "clipboardMaxEntrySizeKB"
    private let translationTargetLanguageKey = "translationTargetLanguage"

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

    public func translationTargetLanguage() -> String {
        defaults.string(forKey: translationTargetLanguageKey) ?? "en"
    }

    public func setTranslationTargetLanguage(_ value: String) {
        defaults.set(value, forKey: translationTargetLanguageKey)
    }
}
