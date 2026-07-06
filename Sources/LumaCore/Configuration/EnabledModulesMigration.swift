import Foundation

/// Schema v2 migration for legacy all-modules-enabled installs (D-012 / D-022).
public enum EnabledModulesMigration {
    public static func migratedEnabledSet(
        stored: Set<ModuleIdentifier>,
        pinned: Set<ModuleIdentifier>
    ) -> Set<ModuleIdentifier> {
        guard !stored.isEmpty else { return ModuleWarmupDefaults.defaultEnabledModuleIDs }

        let fullCatalog = ModuleWarmupDefaults.defaultEnabledModuleIDs
            .union(ModuleWarmupDefaults.expertDefaultOffModuleIDs)
        let mvp = ModuleWarmupDefaults.defaultEnabledModuleIDs
        let allEnabled = stored.isSuperset(of: fullCatalog)

        if allEnabled {
            return mvp.union(pinned)
        }
        return stored.union(mvp).intersection(fullCatalog)
    }
}
