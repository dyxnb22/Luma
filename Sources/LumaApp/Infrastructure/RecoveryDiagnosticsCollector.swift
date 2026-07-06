import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Builds doctor summaries and diagnostics export payloads outside Commands module gating.
@MainActor
enum RecoveryDiagnosticsCollector {
    private static let mvpP0ModuleIDs: [ModuleIdentifier] = [
        ModuleIdentifier(rawValue: "luma.apps"),
        ModuleIdentifier(rawValue: "luma.clipboard"),
        ModuleIdentifier(rawValue: "luma.notes")
    ]

    static func doctorSummary(
        config: ConfigurationStore,
        accessibility: any AccessibilityClient,
        reminders: RemindersService,
        menuBarTree: MenuBarTreeClientAdapter,
        notesConfigStore: NotesRootConfigStore,
        secretsVault: SecretsVault,
        commandsStore: CommandsStore
    ) async -> LumaDiagnosticsSummary {
        let context = await doctorContext(
            config: config,
            accessibility: accessibility,
            reminders: reminders,
            menuBarTree: menuBarTree,
            notesConfigStore: notesConfigStore,
            secretsVault: secretsVault,
            commandsStore: commandsStore
        )
        return LumaDiagnostics.summarize(
            manifests: BuiltInModules.manifestCatalog(),
            context: context
        )
    }

    static func buildExportPayload(
        config: ConfigurationStore,
        accessibility: any AccessibilityClient,
        reminders: RemindersService,
        menuBarTree: MenuBarTreeClientAdapter,
        notesConfigStore: NotesRootConfigStore,
        secretsVault: SecretsVault,
        commandsStore: CommandsStore,
        breadcrumbs: [String],
        recentErrors: [String] = []
    ) async -> DiagnosticsPayload {
        let context = await doctorContext(
            config: config,
            accessibility: accessibility,
            reminders: reminders,
            menuBarTree: menuBarTree,
            notesConfigStore: notesConfigStore,
            secretsVault: secretsVault,
            commandsStore: commandsStore
        )
        let manifests = BuiltInModules.manifestCatalog()
        let enabled = await config.enabledModules() ?? ModuleWarmupDefaults.defaultEnabledModuleIDs
        let manifestIDs = Set(manifests.map(\.identifier))
        let knownEnabled = enabled.intersection(manifestIDs)
        let crashLogPath = CrashLogBuffer.standardFileURL?.path
        let crashLogStatus = await CrashLogBuffer.shared.fileWriteStatus()

        return DiagnosticsExport.buildPayload(
            latencyP95: LatencyTelemetry.shared.currentP95(),
            breadcrumbs: breadcrumbs,
            platform: DiagnosticsPayload.PlatformInfo(
                osVersion: ProcessInfo.processInfo.operatingSystemVersionString,
                screenCount: NSScreen.screens.count,
                presentationScreenName: LumaPresentationScreen.current()?.localizedName
            ),
            modules: DiagnosticsPayload.ModuleInfo(
                enabledCount: knownEnabled.count,
                totalCount: manifests.count,
                defaultEnabledCount: manifests.filter(\.defaultEnabled).count,
                enabledModuleIDs: knownEnabled.map(\.rawValue).sorted(),
                mvpCoreModuleStatus: Self.mvpCoreModuleStatus(enabled: knownEnabled)
            ),
            permissions: DiagnosticsPayload.PermissionsInfo(
                accessibilityTrusted: context.accessibilityTrusted,
                remindersAuthorization: remindersAuthorizationLabel(context.remindersAuthorization),
                hotkeyRegistered: context.hotkeyRegistered
            ),
            recentErrors: recentErrors,
            corruptConfigFiles: context.corruptConfigFiles,
            crashLogPath: crashLogPath,
            crashLogWriteStatus: crashLogStatus
        )
    }

    private static func mvpCoreModuleStatus(
        enabled: Set<ModuleIdentifier>
    ) -> [DiagnosticsPayload.MVPCoreModuleStatus] {
        mvpP0ModuleIDs.map { moduleID in
            DiagnosticsPayload.MVPCoreModuleStatus(
                moduleID: moduleID.rawValue,
                enabled: enabled.contains(moduleID)
            )
        }
    }

    private static func doctorContext(
        config: ConfigurationStore,
        accessibility: any AccessibilityClient,
        reminders: RemindersService,
        menuBarTree: MenuBarTreeClientAdapter,
        notesConfigStore: NotesRootConfigStore,
        secretsVault: SecretsVault,
        commandsStore: CommandsStore
    ) async -> LumaDoctorContext {
        let axTrusted = await accessibility.isTrusted()
        let notesConfig = await notesConfigStore.load()
        let notesRootConfigured = notesConfig.root != nil
        let notesRootReadable: Bool = {
            guard let root = notesConfig.root else { return true }
            return FileManager.default.isReadableFile(atPath: root.path)
        }()
        var corruptFiles = ConfigCorruptionRegistry.snapshot()
        if await commandsStore.loadWasCorrupt() {
            corruptFiles.append("commands.json")
        }
        let manifests = BuiltInModules.manifestCatalog()
        let enabled = await config.enabledModules() ?? ModuleWarmupDefaults.defaultEnabledModuleIDs
        let pinned = await config.pinnedModuleIDs()
        return LumaDoctorContext(
            accessibilityTrusted: axTrusted,
            remindersAuthorization: await reminders.authorization(),
            notesRootConfigured: notesRootConfigured,
            notesRootReadable: notesRootReadable,
            enabledModuleCount: enabled.count,
            totalModuleCount: manifests.count,
            menuItemsCachedCount: await menuBarTree.staleMenuItemCountForFrontmost(),
            hotkeyRegistered: LauncherRuntimeState.hotkeyRegistered,
            commandsConfigValid: !(await commandsStore.loadWasCorrupt()),
            corruptConfigFiles: corruptFiles,
            secretsMetadataCorrupt: await secretsVault.metadataLoadWasCorrupt(),
            secretsLocked: !(await secretsVault.unlocked()),
            clipboardEntryCount: Self.clipboardEntryCount(),
            warmupTimeoutCount: LauncherRuntimeState.warmupTimeoutCount,
            latencyP95Milliseconds: LatencyTelemetry.shared.currentP95(),
            enabledPinnedConsistent: pinned.isSubset(of: enabled)
        )
    }

    private static func clipboardEntryCount() -> Int? {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        let url = base?.appendingPathComponent("Luma/clipboard-history.json")
        guard let url,
              let data = try? Data(contentsOf: url),
              let entries = try? JSONDecoder().decode([ClipboardEntry].self, from: data) else {
            return nil
        }
        return entries.count
    }

    private static func remindersAuthorizationLabel(_ auth: RemindersAuthorization?) -> String? {
        guard let auth else { return "unknown" }
        switch auth {
        case .authorized: return "authorized"
        case .denied: return "denied"
        case .notDetermined: return "notDetermined"
        }
    }
}
