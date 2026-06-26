import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class PermissionBannerController {
    let bannerView = NSView()
    private let config: ConfigurationStore
    private let label = NSTextField(labelWithString: "")
    private var heightConstraint: NSLayoutConstraint?
    private var pollingTask: Task<Void, Never>?

    init(config: ConfigurationStore) {
        self.config = config
    }

    func install(in parent: NSView) {
        bannerView.wantsLayer = true
        bannerView.layer?.backgroundColor = NSColor.systemYellow.withAlphaComponent(0.12).cgColor
        bannerView.layer?.cornerRadius = 8
        bannerView.layer?.cornerCurve = .continuous
        bannerView.translatesAutoresizingMaskIntoConstraints = false

        label.stringValue = "Accessibility permission needed for window focus features."
        label.font = .systemFont(ofSize: 12, weight: .medium)
        label.textColor = .labelColor
        label.translatesAutoresizingMaskIntoConstraints = false

        let actionButton = NSButton(title: "Grant", target: self, action: #selector(grantPermission))
        actionButton.bezelStyle = .rounded
        actionButton.font = .systemFont(ofSize: 12, weight: .medium)
        actionButton.translatesAutoresizingMaskIntoConstraints = false

        bannerView.addSubview(label)
        bannerView.addSubview(actionButton)
        parent.addSubview(bannerView)

        heightConstraint = bannerView.heightAnchor.constraint(equalToConstant: 0)
        NSLayoutConstraint.activate([
            bannerView.leadingAnchor.constraint(equalTo: parent.leadingAnchor, constant: 20),
            bannerView.trailingAnchor.constraint(equalTo: parent.trailingAnchor, constant: -20),
            bannerView.bottomAnchor.constraint(equalTo: parent.bottomAnchor, constant: -8),
            heightConstraint!,

            label.leadingAnchor.constraint(equalTo: bannerView.leadingAnchor, constant: 12),
            label.centerYAnchor.constraint(equalTo: bannerView.centerYAnchor),

            actionButton.trailingAnchor.constraint(equalTo: bannerView.trailingAnchor, constant: -8),
            actionButton.centerYAnchor.constraint(equalTo: bannerView.centerYAnchor)
        ])
        refresh()
    }

    func refresh() {
        Task { await refreshAsync() }
    }

    func startPollingIfNeeded() {
        Task { await startPollingIfNeededAsync() }
    }

    private func refreshAsync() async {
        let enabled = await resolvedEnabledModules()
        guard BuiltInModules.enabledModulesRequireAccessibility(enabled) else {
            heightConstraint?.constant = 0
            bannerView.isHidden = true
            return
        }
        let granted = AXService.isProcessTrusted()
        heightConstraint?.constant = granted ? 0 : 36
        bannerView.isHidden = granted
        label.stringValue = granted
            ? ""
            : "Accessibility permission not active for this build. If Luma is enabled, toggle it off and on."
        if granted {
            stopPolling()
        }
    }

    private func startPollingIfNeededAsync() async {
        let enabled = await resolvedEnabledModules()
        guard BuiltInModules.enabledModulesRequireAccessibility(enabled),
              !AXService.isProcessTrusted() else { return }
        pollingTask?.cancel()
        pollingTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(3))
                await self?.refreshAsync()
            }
        }
    }

    func stopPolling() {
        pollingTask?.cancel()
        pollingTask = nil
    }

    @objc private func grantPermission() {
        AXService.requestPermission()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
        refresh()
    }

    private func resolvedEnabledModules() async -> Set<ModuleIdentifier> {
        let defaultEnabled = Set(
            BuiltInModules.makeAll()
                .filter { type(of: $0).manifest.defaultEnabled }
                .map { type(of: $0).manifest.identifier }
        )
        return await config.enabledModules() ?? defaultEnabled
    }
}
