import AppKit
import SwiftUI
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

struct SettingsSnapshot {
    var enabledModules: Set<ModuleIdentifier>
    var pinnedModuleIDs: Set<ModuleIdentifier>
    var warmupPolicy: WarmupPolicy
    var clipboardMaxEntries: Int
    var clipboardMaxAgeDays: Int
    var clipboardMaxEntrySizeKB: Int
    var clipboardHistoryEnabled: Bool
    var clipboardIgnoredBundleIDs: [String]
    var clipboardPasteBehavior: String
    var translationTargetLanguage: String
    var secretsAutoClearSeconds: Int
    var secretsRelockTimeoutSeconds: Int
    var secretsRequireUnlockOnLaunch: Bool
    var latencyHUDEnabled: Bool
    let modules: [(id: ModuleIdentifier, name: String)]
}

@MainActor
final class SettingsWindowController {
    private var window: NSWindow?
    private let config: ConfigurationStore
    private let usage: PersistentUsageTracker
    private let onModulesChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    private let onPinnedChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    private let onClipboardSettingsChanged: @MainActor (SettingsSnapshot) -> Void
    private let onSecretsSettingsChanged: @MainActor (Int, Int) -> Void
    private let onLatencyHUDChanged: @MainActor (Bool) -> Void

    init(
        config: ConfigurationStore,
        usage: PersistentUsageTracker,
        onModulesChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
        onPinnedChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
        onClipboardSettingsChanged: @escaping @MainActor (SettingsSnapshot) -> Void,
        onSecretsSettingsChanged: @escaping @MainActor (Int, Int) -> Void,
        onLatencyHUDChanged: @escaping @MainActor (Bool) -> Void
    ) {
        self.config = config
        self.usage = usage
        self.onModulesChanged = onModulesChanged
        self.onPinnedChanged = onPinnedChanged
        self.onClipboardSettingsChanged = onClipboardSettingsChanged
        self.onSecretsSettingsChanged = onSecretsSettingsChanged
        self.onLatencyHUDChanged = onLatencyHUDChanged
    }

    func show() {
        Task { @MainActor in
            let snapshot = await makeSnapshot()
            if let window {
                if let hosting = window.contentViewController as? NSHostingController<SettingsSwiftUIView> {
                    hosting.rootView = makeSwiftUIView(snapshot: snapshot)
                }
                present(window)
                return
            }

            let hosting = NSHostingController(rootView: makeSwiftUIView(snapshot: snapshot))
            hosting.sizingOptions = [.preferredContentSize, .intrinsicContentSize]
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 720, height: 520),
                styleMask: [.titled, .closable, .miniaturizable, .resizable],
                backing: .buffered,
                defer: false
            )
            window.title = "Luma Settings"
            window.minSize = NSSize(width: 680, height: 480)
            window.center()
            window.contentViewController = hosting
            window.isReleasedWhenClosed = false
            self.window = window
            present(window)
        }
    }

    private func present(_ window: NSWindow) {
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
        window.orderFrontRegardless()
    }

    private func makeSwiftUIView(snapshot: SettingsSnapshot) -> SettingsSwiftUIView {
        SettingsSwiftUIView(
            snapshot: snapshot,
            config: config,
            usage: usage,
            onModulesChanged: onModulesChanged,
            onPinnedChanged: onPinnedChanged,
            onClipboardSettingsChanged: onClipboardSettingsChanged,
            onSecretsSettingsChanged: onSecretsSettingsChanged,
            onLatencyHUDChanged: onLatencyHUDChanged
        )
    }

    private func makeSnapshot() async -> SettingsSnapshot {
        let modules = BuiltInModules.makeAll().map { (type(of: $0).manifest.identifier, type(of: $0).manifest.displayName) }
        let defaultEnabled = Set(BuiltInModules.makeAll().filter { type(of: $0).manifest.defaultEnabled }.map { type(of: $0).manifest.identifier })
        return SettingsSnapshot(
            enabledModules: await config.enabledModules() ?? defaultEnabled,
            pinnedModuleIDs: await config.pinnedModuleIDs(),
            warmupPolicy: await config.warmupPolicy(),
            clipboardMaxEntries: await config.clipboardMaxEntries(),
            clipboardMaxAgeDays: await config.clipboardMaxAgeDays(),
            clipboardMaxEntrySizeKB: await config.clipboardMaxEntrySizeKB(),
            clipboardHistoryEnabled: await config.clipboardHistoryEnabled(),
            clipboardIgnoredBundleIDs: await config.clipboardIgnoredBundleIDs(),
            clipboardPasteBehavior: await config.clipboardPasteBehavior(),
            translationTargetLanguage: await config.translationTargetLanguage(),
            secretsAutoClearSeconds: await config.secretsAutoClearSeconds(),
            secretsRelockTimeoutSeconds: await config.secretsRelockTimeoutSeconds(),
            secretsRequireUnlockOnLaunch: await config.secretsRequireUnlockOnLaunch(),
            latencyHUDEnabled: await config.latencyHUDEnabled(),
            modules: modules
        )
    }
}
