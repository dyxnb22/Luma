import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class PermissionBannerController {
    let bannerView = NSView()
    var onOpenSettings: (() -> Void)?
    private static let defaultEnabledModuleIDs = ModuleRegistry.defaultEnabledModuleIDs

    private let config: ConfigurationStore
    private let label = NSTextField(labelWithString: "")
    private var heightConstraint: NSLayoutConstraint?
    private var pollingTask: Task<Void, Never>?
    private var refreshTask: Task<Void, Never>?
    private var lastContext = AccessibilityGuidanceContext(surface: .none)
    private var lastAppliedContext: AccessibilityGuidanceContext?
    private var lastEnabledModules: Set<ModuleIdentifier>?
    private(set) var isBannerVisible = false

    init(config: ConfigurationStore) {
        self.config = config
    }

    func install(in parent: NSView, above anchorView: NSView) {
        let chromeView = NSView()
        chromeView.wantsLayer = true
        chromeView.layer?.backgroundColor = NSColor.systemYellow.withAlphaComponent(0.12).cgColor
        chromeView.layer?.cornerRadius = 8
        chromeView.layer?.cornerCurve = .continuous
        chromeView.translatesAutoresizingMaskIntoConstraints = false

        bannerView.translatesAutoresizingMaskIntoConstraints = false
        bannerView.addSubview(chromeView)

        label.stringValue = L10n.tr("permission.banner.initial")
        label.font = .systemFont(ofSize: 12, weight: .medium)
        label.textColor = .labelColor
        label.translatesAutoresizingMaskIntoConstraints = false

        let actionButton = NSButton(title: L10n.tr("permission.banner.grant"), target: self, action: #selector(grantPermission))
        actionButton.bezelStyle = .rounded
        actionButton.font = .systemFont(ofSize: 12, weight: .medium)
        actionButton.translatesAutoresizingMaskIntoConstraints = false

        let settingsButton = NSButton(title: L10n.tr("permission.banner.settings"), target: self, action: #selector(openSettings))
        settingsButton.bezelStyle = .rounded
        settingsButton.font = .systemFont(ofSize: 12, weight: .medium)
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        bannerView.addSubview(label)
        bannerView.addSubview(settingsButton)
        bannerView.addSubview(actionButton)
        parent.addSubview(bannerView)

        heightConstraint = bannerView.heightAnchor.constraint(equalToConstant: 0)
        NSLayoutConstraint.activate([
            chromeView.topAnchor.constraint(equalTo: bannerView.topAnchor),
            chromeView.leadingAnchor.constraint(equalTo: bannerView.leadingAnchor),
            chromeView.trailingAnchor.constraint(equalTo: bannerView.trailingAnchor),
            chromeView.bottomAnchor.constraint(equalTo: bannerView.bottomAnchor),

            bannerView.leadingAnchor.constraint(equalTo: parent.leadingAnchor, constant: 20),
            bannerView.trailingAnchor.constraint(equalTo: parent.trailingAnchor, constant: -20),
            bannerView.bottomAnchor.constraint(equalTo: anchorView.topAnchor, constant: -8),
            heightConstraint!,

            label.leadingAnchor.constraint(equalTo: bannerView.leadingAnchor, constant: 12),
            label.centerYAnchor.constraint(equalTo: bannerView.centerYAnchor),
            label.trailingAnchor.constraint(lessThanOrEqualTo: settingsButton.leadingAnchor, constant: -8),

            actionButton.trailingAnchor.constraint(equalTo: bannerView.trailingAnchor, constant: -8),
            actionButton.centerYAnchor.constraint(equalTo: bannerView.centerYAnchor),

            settingsButton.trailingAnchor.constraint(equalTo: actionButton.leadingAnchor, constant: -6),
            settingsButton.centerYAnchor.constraint(equalTo: bannerView.centerYAnchor)
        ])
        refresh(context: .init(surface: .none))
    }

    func refresh(context: AccessibilityGuidanceContext) {
        if context == lastAppliedContext { return }
        lastContext = context
        refreshTask?.cancel()
        LauncherPerfCounters.increment(.permissionRefresh)
        if context.surface == .none {
            applyBannerVisibility(false, for: context)
            return
        }
        refreshTask = Task { [weak self] in
            await self?.refreshAsync(context: context)
        }
    }

    func startPollingIfNeeded() {
        guard isBannerVisible else {
            stopPolling()
            return
        }
        Task { await startPollingIfNeededAsync() }
    }

    private func refreshAsync(context: AccessibilityGuidanceContext) async {
        let enabled: Set<ModuleIdentifier>
        if let cached = lastEnabledModules {
            enabled = cached
        } else {
            enabled = await resolvedEnabledModules()
            lastEnabledModules = enabled
        }
        let shouldShow = AccessibilityGuidancePolicy.shouldShowBanner(
            context: context,
            enabledModules: enabled
        )
        applyBannerVisibility(shouldShow, for: context)
        if shouldShow { await startPollingIfNeededAsync() }
    }

    func invalidateEnabledModuleCache() {
        lastEnabledModules = nil
        lastAppliedContext = nil
    }

    private func applyBannerVisibility(_ shouldShow: Bool, for context: AccessibilityGuidanceContext) {
        isBannerVisible = shouldShow
        heightConstraint?.constant = shouldShow ? 36 : 0
        bannerView.isHidden = !shouldShow
        label.stringValue = shouldShow ? L10n.tr("permission.banner.inactive") : ""
        if !shouldShow { stopPolling() }
        lastAppliedContext = context
    }

    private func startPollingIfNeededAsync() async {
        guard isBannerVisible, !AXService.isProcessTrusted() else {
            stopPolling()
            return
        }
        pollingTask?.cancel()
        pollingTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(3))
                guard let self else { return }
                await self.refreshAsync(context: self.lastContext)
            }
        }
    }

    func stopPolling() {
        pollingTask?.cancel()
        pollingTask = nil
    }

    @objc nonisolated private func grantPermission() {
        Task { @MainActor [weak self] in
            self?.grantPermissionOnMainActor()
        }
    }

    @objc nonisolated private func openSettings() {
        Task { @MainActor [weak self] in
            self?.openSettingsOnMainActor()
        }
    }

    @MainActor
    private func grantPermissionOnMainActor() {
        AXService.requestPermission()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            Task {
                await SystemSettingsOpener.open(
                    url,
                    onFailure: { message in
                        LauncherEnvironment.current?.showStatus(message)
                    }
                )
            }
        }
        lastAppliedContext = nil
        refresh(context: lastContext)
    }

    @MainActor
    private func openSettingsOnMainActor() {
        onOpenSettings?()
    }

    private func resolvedEnabledModules() async -> Set<ModuleIdentifier> {
        await config.enabledModules() ?? Self.defaultEnabledModuleIDs
    }
}
