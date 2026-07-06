import Foundation
import LumaCore
import Testing

@Test func enabledModulesMigrationV2TrimsAllEnabledLegacyInstall() {
    var all = ModuleWarmupDefaults.defaultEnabledModuleIDs
    all.formUnion(ModuleWarmupDefaults.expertDefaultOffModuleIDs)
    let migrated = EnabledModulesMigration.migratedEnabledSet(stored: all, pinned: [])
    #expect(migrated == ModuleWarmupDefaults.defaultEnabledModuleIDs)
}

@Test func enabledModulesMigrationV2PreservesSingleDisabledExpertModule() {
    var stored = ModuleWarmupDefaults.defaultEnabledModuleIDs
    stored.formUnion(ModuleWarmupDefaults.expertDefaultOffModuleIDs)
    stored.remove(ModuleIdentifier(rawValue: "luma.media"))
    let migrated = EnabledModulesMigration.migratedEnabledSet(stored: stored, pinned: [])
    #expect(migrated.contains(ModuleIdentifier(rawValue: "luma.media")) == false)
    #expect(migrated.contains(ModuleIdentifier(rawValue: "luma.projects")))
}

@Test func enabledModulesMigrationV2RemovesUnknownModuleIDs() {
    var stored = ModuleWarmupDefaults.defaultEnabledModuleIDs
    stored.insert(ModuleIdentifier(rawValue: "luma.legacy-unknown"))
    let migrated = EnabledModulesMigration.migratedEnabledSet(stored: stored, pinned: [])
    #expect(!migrated.contains(ModuleIdentifier(rawValue: "luma.legacy-unknown")))
}

@Test func enabledModulesMigrationV2MVPOnlyStoredDoesNotExpandToAllExperts() {
    let stored = ModuleWarmupDefaults.defaultEnabledModuleIDs
    let migrated = EnabledModulesMigration.migratedEnabledSet(stored: stored, pinned: [])
    #expect(migrated == ModuleWarmupDefaults.defaultEnabledModuleIDs)
}

@Test func enabledModulesMigrationV2PreservesExplicitExpertWhenPinned() {
    var stored = ModuleWarmupDefaults.defaultEnabledModuleIDs
    stored.insert(ModuleIdentifier(rawValue: "luma.projects"))
    let pinned: Set<ModuleIdentifier> = [ModuleIdentifier(rawValue: "luma.projects")]
    let migrated = EnabledModulesMigration.migratedEnabledSet(stored: stored, pinned: pinned)
    #expect(migrated.contains(ModuleIdentifier(rawValue: "luma.projects")))
}
