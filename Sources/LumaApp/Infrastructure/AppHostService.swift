import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class AppHostService: HostClient {
    private let config: ConfigurationStore
    private let accessibility: any AccessibilityClient
    private let reminders: RemindersService
    private let menuBarTree: MenuBarTreeClientAdapter
    private let notesConfigStore: NotesRootConfigStore
    private let secretsVault: SecretsVault
    private let commandsStore: CommandsStore
    private let onOpenSettings: () -> Void
    private let onReloadModules: () -> Void

    init(
        config: ConfigurationStore,
        accessibility: any AccessibilityClient,
        reminders: RemindersService,
        menuBarTree: MenuBarTreeClientAdapter,
        notesConfigStore: NotesRootConfigStore,
        secretsVault: SecretsVault,
        commandsStore: CommandsStore,
        onOpenSettings: @escaping () -> Void,
        onReloadModules: @escaping () -> Void
    ) {
        self.config = config
        self.accessibility = accessibility
        self.reminders = reminders
        self.menuBarTree = menuBarTree
        self.notesConfigStore = notesConfigStore
        self.secretsVault = secretsVault
        self.commandsStore = commandsStore
        self.onOpenSettings = onOpenSettings
        self.onReloadModules = onReloadModules
    }

    func openSettings() async {
        onOpenSettings()
    }

    func reloadModules() async {
        onReloadModules()
    }

    func quitHost() async {
        NSApp.terminate(nil)
    }

    func runDoctor() async {
        let summary = await RecoveryDiagnosticsCollector.doctorSummary(
            config: config,
            accessibility: accessibility,
            reminders: reminders,
            menuBarTree: menuBarTree,
            notesConfigStore: notesConfigStore,
            secretsVault: secretsVault,
            commandsStore: commandsStore
        )
        RecoveryDiagnosticsPresenter.showDoctorSummary(summary)
    }

    func exportDiagnostics() async throws -> URL {
        let breadcrumbs = await CrashLogBuffer.shared.all()
        let payload = await RecoveryDiagnosticsCollector.buildExportPayload(
            config: config,
            accessibility: accessibility,
            reminders: reminders,
            menuBarTree: menuBarTree,
            notesConfigStore: notesConfigStore,
            secretsVault: secretsVault,
            commandsStore: commandsStore,
            breadcrumbs: breadcrumbs
        )
        return try DiagnosticsExport.writePayload(payload)
    }
}
