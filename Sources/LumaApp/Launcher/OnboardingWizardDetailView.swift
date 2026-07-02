import AppKit
import LumaCore
import LumaServices

@MainActor
final class OnboardingWizardDetailView: ModuleDetailView {
    let moduleTitle: String
    let detailView: NSView
    let usesSharedTopBar = true

    private struct Step {
        let titleKey: String.LocalizationValue
        let bodyKey: String.LocalizationValue
        let showsGrantAccess: Bool
        let showsOpenSettings: Bool
    }

    private let steps: [Step] = [
        Step(titleKey: "onboarding.step.welcome.title", bodyKey: "onboarding.step.welcome.body", showsGrantAccess: false, showsOpenSettings: false),
        Step(titleKey: "onboarding.step.hotkey.title", bodyKey: "onboarding.step.hotkey.body", showsGrantAccess: false, showsOpenSettings: false),
        Step(titleKey: "onboarding.step.accessibility.title", bodyKey: "onboarding.step.accessibility.body", showsGrantAccess: true, showsOpenSettings: false),
        Step(titleKey: "onboarding.step.modules.title", bodyKey: "onboarding.step.modules.body", showsGrantAccess: false, showsOpenSettings: true),
        Step(titleKey: "onboarding.step.help.title", bodyKey: "onboarding.step.help.body", showsGrantAccess: false, showsOpenSettings: false),
        Step(titleKey: "onboarding.step.done.title", bodyKey: "onboarding.step.done.body", showsGrantAccess: false, showsOpenSettings: false)
    ]

    private let onOpenSettings: () -> Void
    private let onComplete: () -> Void

    private var stepIndex = 0
    private let titleLabel = NSTextField(labelWithString: "")
    private let bodyLabel = NSTextField(wrappingLabelWithString: "")
    private let progressLabel = NSTextField(labelWithString: "")
    private let grantButton = NSButton()
    private let settingsButton = NSButton()
    private let backButton = NSButton()
    private let nextButton = NSButton()
    private let skipButton = NSButton()

    init(onOpenSettings: @escaping () -> Void, onComplete: @escaping () -> Void) {
        self.moduleTitle = L10n.tr("onboarding.title")
        self.onOpenSettings = onOpenSettings
        self.onComplete = onComplete
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        setup(chrome: chrome)
        renderStep()
    }

    func activate() {}
    func deactivate() {}

    private func setup(chrome: BaseDetailContainer) {
        titleLabel.font = .systemFont(ofSize: 16, weight: .semibold)
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        bodyLabel.font = .systemFont(ofSize: 13)
        bodyLabel.textColor = .secondaryLabelColor
        bodyLabel.preferredMaxLayoutWidth = 520
        bodyLabel.translatesAutoresizingMaskIntoConstraints = false

        progressLabel.font = TypographyTokens.monoMeta()
        progressLabel.textColor = .tertiaryLabelColor
        progressLabel.translatesAutoresizingMaskIntoConstraints = false

        grantButton.title = L10n.tr("onboarding.button.grantAccess")
        grantButton.bezelStyle = .rounded
        grantButton.target = self
        grantButton.action = #selector(grantAccessTapped)
        grantButton.translatesAutoresizingMaskIntoConstraints = false

        settingsButton.title = L10n.tr("setup.openSettings")
        settingsButton.bezelStyle = .rounded
        settingsButton.target = self
        settingsButton.action = #selector(openSettingsTapped)
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        backButton.title = L10n.tr("onboarding.button.back")
        backButton.bezelStyle = .rounded
        backButton.target = self
        backButton.action = #selector(backTapped)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        nextButton.title = L10n.tr("onboarding.button.next")
        GeekUIKit.stylePrimaryButton(nextButton)
        nextButton.target = self
        nextButton.action = #selector(nextTapped)
        nextButton.translatesAutoresizingMaskIntoConstraints = false

        skipButton.title = L10n.tr("onboarding.button.skip")
        skipButton.bezelStyle = .inline
        skipButton.target = self
        skipButton.action = #selector(skipTapped)
        skipButton.translatesAutoresizingMaskIntoConstraints = false

        let content = NSView()
        content.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(progressLabel)
        content.addSubview(titleLabel)
        content.addSubview(bodyLabel)
        content.addSubview(grantButton)
        content.addSubview(settingsButton)

        let footer = NSStackView(views: [skipButton, backButton, nextButton])
        footer.orientation = .horizontal
        footer.spacing = 8
        footer.translatesAutoresizingMaskIntoConstraints = false

        chrome.setContent(content, embedInScroll: false)
        chrome.setFooter(footer, height: LauncherChromeTokens.detailFooterHeight)

        NSLayoutConstraint.activate([
            progressLabel.topAnchor.constraint(equalTo: content.topAnchor, constant: 8),
            progressLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 4),

            titleLabel.topAnchor.constraint(equalTo: progressLabel.bottomAnchor, constant: 12),
            titleLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 4),
            titleLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -4),

            bodyLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 10),
            bodyLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 4),
            bodyLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -4),

            grantButton.topAnchor.constraint(equalTo: bodyLabel.bottomAnchor, constant: 16),
            grantButton.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 4),

            settingsButton.centerYAnchor.constraint(equalTo: grantButton.centerYAnchor),
            settingsButton.leadingAnchor.constraint(equalTo: grantButton.trailingAnchor, constant: 8),
            settingsButton.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -8)
        ])
    }

    private func renderStep() {
        let step = steps[stepIndex]
        titleLabel.stringValue = L10n.tr(step.titleKey)
        bodyLabel.stringValue = L10n.tr(step.bodyKey)
        progressLabel.stringValue = "\(stepIndex + 1) / \(steps.count)"
        grantButton.isHidden = !step.showsGrantAccess
        settingsButton.isHidden = !step.showsOpenSettings
        backButton.isEnabled = stepIndex > 0
        nextButton.title = stepIndex == steps.count - 1
            ? L10n.tr("onboarding.button.finish")
            : L10n.tr("onboarding.button.next")
    }

    @objc private func nextTapped() {
        if stepIndex >= steps.count - 1 {
            onComplete()
            return
        }
        stepIndex += 1
        renderStep()
    }

    @objc private func backTapped() {
        guard stepIndex > 0 else { return }
        stepIndex -= 1
        renderStep()
    }

    @objc private func skipTapped() {
        onComplete()
    }

    @objc private func grantAccessTapped() {
        AXService.requestPermission()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
    }

    @objc private func openSettingsTapped() {
        onOpenSettings()
    }
}
