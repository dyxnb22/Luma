import AppKit
import Foundation
import LumaCore
import LumaInfrastructure

@MainActor
final class AppHostService: HostClient {
    private let onOpenSettings: () -> Void
    private let onReloadModules: () -> Void

    init(
        onOpenSettings: @escaping () -> Void,
        onReloadModules: @escaping () -> Void
    ) {
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

    func exportDiagnostics() async throws -> URL {
        let breadcrumbs = await CrashLogBuffer.shared.all()
        let latencyP95 = LatencyTelemetry.shared.currentP95()
        return try DiagnosticsExport.exportToLogsDirectory(
            latencyP95: latencyP95,
            breadcrumbs: breadcrumbs
        )
    }
}
