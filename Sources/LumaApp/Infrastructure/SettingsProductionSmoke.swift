import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules

/// Signed-app smoke for Settings open/save using production wiring.
/// Triggered only when `LUMA_QA_SETTINGS=1`; restores toggled values afterward.
@MainActor
enum SettingsProductionSmoke {
    struct Report: Codable {
        let generatedAt: String
        let commandsModuleDefaultOff: Bool
        let latencyHUDPersisted: Bool
        let latencyHUDRestored: Bool
        let clipboardMaxEntriesPersisted: Bool
        let clipboardMaxEntriesRestored: Bool
        let hotkeyLoadMatchesDefault: Bool
        let hotkeySaveIsNoOp: Bool
        let settingsWindowSingleInstance: Bool
        let corruptConfigFilesInRegistry: [String]
        let userDefaultsSuite: String
    }

    static func run(
        config: ConfigurationStore,
        settingsWindowController: SettingsWindowController
    ) async {
        let originalLatencyHUD = await config.latencyHUDEnabled()
        let originalClipboardMax = await config.clipboardMaxEntries()
        var latencyHUDPersisted = false
        var latencyHUDRestored = false
        var clipboardMaxEntriesPersisted = false
        var clipboardMaxEntriesRestored = false
        var settingsWindowSingleInstance = false

        let toggledHUD = !originalLatencyHUD
        await config.setLatencyHUDEnabled(toggledHUD)
        latencyHUDPersisted = await config.latencyHUDEnabled() == toggledHUD

        let testMax = originalClipboardMax == 501 ? 502 : 501
        await config.setClipboardMaxEntries(testMax)
        clipboardMaxEntriesPersisted = await config.clipboardMaxEntries() == testMax

        settingsWindowController.show()
        var windowCount = 0
        for _ in 0..<30 {
            try? await Task.sleep(for: .milliseconds(100))
            windowCount = NSApp.windows.filter { $0.title == "Luma Settings" }.count
            if windowCount == 1 { break }
        }
        settingsWindowController.show()
        try? await Task.sleep(for: .milliseconds(500))
        let settingsWindows = NSApp.windows.filter { $0.title == "Luma Settings" }
        settingsWindowSingleInstance = settingsWindows.count == 1
        settingsWindows.first?.close()

        await config.setClipboardMaxEntries(originalClipboardMax)
        clipboardMaxEntriesRestored = await config.clipboardMaxEntries() == originalClipboardMax
        await config.setLatencyHUDEnabled(originalLatencyHUD)
        latencyHUDRestored = await config.latencyHUDEnabled() == originalLatencyHUD

        let probeCombo = KeyCombo(virtualKeyCode: 17, carbonModifiers: 1 << 8)
        HotkeyConfig.save(probeCombo)
        let hotkeySaveIsNoOp = HotkeyConfig.load() == HotkeyConfig.defaultCombo
        HotkeyConfig.resetToDefault()

        let commandsDefaultOff = ModuleRegistry.bundle(for: .commands)?.manifest.defaultEnabled == false

        let report = Report(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            commandsModuleDefaultOff: commandsDefaultOff,
            latencyHUDPersisted: latencyHUDPersisted,
            latencyHUDRestored: latencyHUDRestored,
            clipboardMaxEntriesPersisted: clipboardMaxEntriesPersisted,
            clipboardMaxEntriesRestored: clipboardMaxEntriesRestored,
            hotkeyLoadMatchesDefault: HotkeyConfig.load() == HotkeyConfig.defaultCombo,
            hotkeySaveIsNoOp: hotkeySaveIsNoOp,
            settingsWindowSingleInstance: settingsWindowSingleInstance,
            corruptConfigFilesInRegistry: ConfigCorruptionRegistry.snapshot(),
            userDefaultsSuite: Bundle.main.bundleIdentifier ?? "unknown"
        )
        write(report)
    }

    private static func write(_ report: Report) {
        guard let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma", isDirectory: true) else {
            CrashLogRecording.record("settings.smoke.failed reason=logs-directory-unavailable")
            return
        }
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let url = directory.appendingPathComponent("settings-smoke.json")
            try JSONEncoder().encode(report).write(to: url, options: .atomic)
        } catch {
            CrashLogRecording.record("settings.smoke.failed error=\(error.localizedDescription)")
        }
    }
}
