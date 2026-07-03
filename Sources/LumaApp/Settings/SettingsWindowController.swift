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
    private var window: LumaWindow?
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

    func show(section: SettingsSection = .general) {
        Task { @MainActor in
            let snapshot = await makeSnapshot()
            if let window {
                if let hosting = window.contentViewController as? NSHostingController<SettingsSwiftUIView> {
                    hosting.rootView = makeSwiftUIView(snapshot: snapshot, initialSection: section)
                }
                present(window)
                return
            }

            let hosting = NSHostingController(rootView: makeSwiftUIView(snapshot: snapshot, initialSection: section))
            hosting.sizingOptions = [.preferredContentSize, .intrinsicContentSize]
            let window = LumaWindow(
                contentRect: NSRect(x: 0, y: 0, width: 720, height: 520),
                styleMask: [.titled, .closable, .miniaturizable, .resizable],
                backing: .buffered,
                defer: false
            )
            window.title = "Luma Settings"
            window.minSize = NSSize(width: 680, height: 480)
            window.contentViewController = hosting
            centerOnPresentationScreen(window)
            window.isReleasedWhenClosed = false
            self.window = window
            present(window)
        }
    }

    private func present(_ window: LumaWindow) {
        centerOnPresentationScreen(window)
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
        window.orderFrontRegardless()
    }

    private func centerOnPresentationScreen(_ window: NSWindow) {
        guard let screen = LumaPresentationScreen.current() else {
            window.center()
            return
        }
        let visible = screen.visibleFrame
        let origin = LauncherPanelGeometry.centeredOrigin(for: window.frame.size, in: visible)
        window.setFrameOrigin(origin)
    }

    private func makeSwiftUIView(snapshot: SettingsSnapshot, initialSection: SettingsSection = .general) -> SettingsSwiftUIView {
        SettingsSwiftUIView(
            snapshot: snapshot,
            config: config,
            usage: usage,
            onModulesChanged: onModulesChanged,
            onPinnedChanged: onPinnedChanged,
            onClipboardSettingsChanged: onClipboardSettingsChanged,
            onSecretsSettingsChanged: onSecretsSettingsChanged,
            onLatencyHUDChanged: onLatencyHUDChanged,
            initialSection: initialSection
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
