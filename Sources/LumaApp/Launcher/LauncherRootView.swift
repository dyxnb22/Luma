import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
private final class FlippedStackView: NSStackView {
    override var isFlipped: Bool { true }
}

@MainActor
final class LauncherRootView: NSView {
    private let glassBackground = NSVisualEffectView()
    private let highlightLayer = CAGradientLayer()
    private let searchBar = LumaSearchBar()
    private let sidebarContainer = NSView()
    private let sidebarStack = FlippedStackView()
    private let contentContainer = NSView()
    private let featureGridView = NSView()
    private let resultsScrollView = NSScrollView()
    private let resultsStackView = FlippedStackView()
    private let featureGridStack = NSStackView()
    private let loadingLabel = NSTextField(labelWithString: "Loading…")
    private let cards: [FeatureCard]
    private let sortedCards: [FeatureCard]
    private let viewModel: LauncherViewModel
    private let actionExecutor: ActionExecutor
    private let appActivationTracker: AppActivationTracker
    private let onDismiss: () -> Void
    private let onActionDismiss: () -> Void
    private var currentItems: [ResultItem] = []
    private var selectedIndex = 0
    private var modulesReady = false
    private var showingResults = false
    private let detailContainer = NSView()
    private let detailTopBar = NSView()
    private let detailTitleLabel = NSTextField(labelWithString: "")
    private var currentDetailObject: (any ModuleDetailView)?
    private var showingDetail = false
    private let permissionBanner = NSView()
    private let permissionBannerLabel = NSTextField(labelWithString: "")
    private var permissionBannerHeight: NSLayoutConstraint?
    private var permissionPollingTask: Task<Void, Never>?
    private var widgetCards: [WidgetFeatureCard] = []
    private var pendingTranslateText: String?

    init(
        cards: [FeatureCard],
        viewModel: LauncherViewModel,
        actionExecutor: ActionExecutor,
        appActivationTracker: AppActivationTracker,
        onDismiss: @escaping () -> Void,
        onActionDismiss: @escaping () -> Void
    ) {
        self.cards = cards
        self.sortedCards = cards.sorted { $0.position.column < $1.position.column }
        self.viewModel = viewModel
        self.actionExecutor = actionExecutor
        self.appActivationTracker = appActivationTracker
        self.onDismiss = onDismiss
        self.onActionDismiss = onActionDismiss
        super.init(frame: .zero)

        searchBar.onEscape = { [weak self] in self?.handleEscape() }
        searchBar.onReturn = { [weak self] in self?.activateFirstSearchResult() }
        searchBar.onTextChange = { [weak self] text in self?.handleTextChange(text) }
        searchBar.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }

        viewModel.onSnapshot = { [weak self] snapshot in
            self?.apply(snapshot: snapshot)
        }

        setupGlassChrome()
        setupLayout()
        setupPermissionBanner()
        startPermissionPolling()
        showHome()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func focusSearchField() {
        searchBar.focus()
    }

    func setModulesReady(_ ready: Bool) {
        modulesReady = ready
        if ready, searchBar.stringValue.isEmpty {
            loadingLabel.isHidden = true
        }
    }

    func showHome(focusSearch: Bool = true) {
        searchBar.stringValue = ""
        if showingDetail {
            showingDetail = false
            currentDetailObject?.deactivate()
            currentDetailObject?.detailView.removeFromSuperview()
            currentDetailObject = nil
            detailContainer.isHidden = true
            detailContainer.alphaValue = 0
        }
        showingResults = false
        renderResults([])
        featureGridView.isHidden = false
        featureGridView.alphaValue = 1
        resultsScrollView.isHidden = true
        resultsScrollView.alphaValue = 0
        renderFeatureCards()
        loadingLabel.isHidden = modulesReady
        refreshOpenApps()
        refreshPermissionBanner()
        if focusSearch {
            focusSearchField()
        }
    }

    func refreshOpenApps() {
        Task { await renderOpenApps() }
    }

    func refreshPermissionStatus() {
        refreshPermissionBanner()
    }

    func onFeatureSelected(_ trigger: String) {
        searchBar.stringValue = trigger
        searchBar.focus()
        handleTextChange(trigger)
    }

    private func setupGlassChrome() {
        wantsLayer = true
        layer?.cornerRadius = 20
        layer?.cornerCurve = .continuous
        layer?.masksToBounds = true
        layer?.borderWidth = 1
        layer?.borderColor = NSColor.white.withAlphaComponent(0.18).cgColor

        glassBackground.material = .underWindowBackground
        glassBackground.blendingMode = .behindWindow
        glassBackground.state = .active
        glassBackground.translatesAutoresizingMaskIntoConstraints = false
        addSubview(glassBackground, positioned: .below, relativeTo: nil)

        NSLayoutConstraint.activate([
            glassBackground.topAnchor.constraint(equalTo: topAnchor),
            glassBackground.leadingAnchor.constraint(equalTo: leadingAnchor),
            glassBackground.trailingAnchor.constraint(equalTo: trailingAnchor),
            glassBackground.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])

        highlightLayer.colors = [
            NSColor.white.withAlphaComponent(0.18).cgColor,
            NSColor.white.withAlphaComponent(0.0).cgColor
        ]
        highlightLayer.locations = [0.0, 0.35]
        highlightLayer.startPoint = CGPoint(x: 0.5, y: 0)
        highlightLayer.endPoint = CGPoint(x: 0.5, y: 1)
        layer?.addSublayer(highlightLayer)
    }

    private func setupPermissionBanner() {
        permissionBanner.wantsLayer = true
        permissionBanner.layer?.backgroundColor = NSColor.systemYellow.withAlphaComponent(0.12).cgColor
        permissionBanner.layer?.cornerRadius = 8
        permissionBanner.layer?.cornerCurve = .continuous
        permissionBanner.translatesAutoresizingMaskIntoConstraints = false

        permissionBannerLabel.stringValue = "Accessibility permission needed for window focus features."
        permissionBannerLabel.font = .systemFont(ofSize: 12, weight: .medium)
        permissionBannerLabel.textColor = .labelColor
        permissionBannerLabel.translatesAutoresizingMaskIntoConstraints = false

        let actionButton = NSButton(title: "Grant", target: self, action: #selector(grantPermission))
        actionButton.bezelStyle = .rounded
        actionButton.font = .systemFont(ofSize: 12, weight: .medium)
        actionButton.translatesAutoresizingMaskIntoConstraints = false

        permissionBanner.addSubview(permissionBannerLabel)
        permissionBanner.addSubview(actionButton)
        addSubview(permissionBanner)

        permissionBannerHeight = permissionBanner.heightAnchor.constraint(equalToConstant: 0)
        NSLayoutConstraint.activate([
            permissionBanner.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            permissionBanner.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            permissionBanner.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -8),
            permissionBannerHeight!,

            permissionBannerLabel.leadingAnchor.constraint(equalTo: permissionBanner.leadingAnchor, constant: 12),
            permissionBannerLabel.centerYAnchor.constraint(equalTo: permissionBanner.centerYAnchor),

            actionButton.trailingAnchor.constraint(equalTo: permissionBanner.trailingAnchor, constant: -8),
            actionButton.centerYAnchor.constraint(equalTo: permissionBanner.centerYAnchor)
        ])
    }

    @objc private func grantPermission() {
        AXService.requestPermission()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
        refreshPermissionBanner()
    }

    private func refreshPermissionBanner() {
        guard BuiltInModules.activeModulesRequireAccessibility() else {
            permissionBannerHeight?.constant = 0
            permissionBanner.isHidden = true
            return
        }
        let granted = AXService.isProcessTrusted()
        permissionBannerHeight?.constant = granted ? 0 : 36
        permissionBanner.isHidden = granted
        permissionBannerLabel.stringValue = granted
            ? ""
            : "Accessibility permission not active for this build. If Luma is enabled, toggle it off and on."
    }

    private func startPermissionPolling() {
        guard BuiltInModules.activeModulesRequireAccessibility() else { return }
        permissionPollingTask?.cancel()
        permissionPollingTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(1))
                await MainActor.run {
                    self?.refreshPermissionBanner()
                }
            }
        }
    }

    deinit {
        permissionPollingTask?.cancel()
    }

    override func layout() {
        super.layout()
        highlightLayer.frame = bounds
    }

    private func setupLayout() {
        searchBar.translatesAutoresizingMaskIntoConstraints = false

        let sidebarHeader = NSTextField(labelWithString: "OPEN APPS")
        sidebarHeader.font = .systemFont(ofSize: 11, weight: .semibold)
        sidebarHeader.textColor = .secondaryLabelColor
        sidebarHeader.translatesAutoresizingMaskIntoConstraints = false

        sidebarStack.orientation = .vertical
        sidebarStack.spacing = 2
        sidebarStack.alignment = .leading
        sidebarStack.translatesAutoresizingMaskIntoConstraints = false

        let sidebarSeparator = NSView()
        sidebarSeparator.wantsLayer = true
        sidebarSeparator.layer?.backgroundColor = NSColor.separatorColor.withAlphaComponent(0.25).cgColor
        sidebarSeparator.translatesAutoresizingMaskIntoConstraints = false

        sidebarContainer.translatesAutoresizingMaskIntoConstraints = false
        sidebarContainer.addSubview(sidebarHeader)
        sidebarContainer.addSubview(sidebarStack)
        sidebarContainer.addSubview(sidebarSeparator)

        contentContainer.translatesAutoresizingMaskIntoConstraints = false

        featureGridStack.orientation = .horizontal
        featureGridStack.spacing = 16
        featureGridStack.alignment = .centerY
        featureGridStack.distribution = .fillEqually
        featureGridStack.translatesAutoresizingMaskIntoConstraints = false

        featureGridView.translatesAutoresizingMaskIntoConstraints = false
        featureGridView.addSubview(featureGridStack)

        resultsStackView.orientation = .vertical
        resultsStackView.spacing = 6
        resultsStackView.alignment = .leading
        resultsStackView.translatesAutoresizingMaskIntoConstraints = false

        resultsScrollView.hasVerticalScroller = true
        resultsScrollView.drawsBackground = false
        resultsScrollView.borderType = .noBorder
        resultsScrollView.translatesAutoresizingMaskIntoConstraints = false
        resultsScrollView.documentView = resultsStackView

        loadingLabel.font = .systemFont(ofSize: 13)
        loadingLabel.textColor = .secondaryLabelColor
        loadingLabel.alignment = .center
        loadingLabel.isHidden = true
        loadingLabel.translatesAutoresizingMaskIntoConstraints = false

        contentContainer.addSubview(featureGridView)
        contentContainer.addSubview(resultsScrollView)
        contentContainer.addSubview(loadingLabel)

        addSubview(searchBar)
        addSubview(sidebarContainer)
        addSubview(contentContainer)

        NSLayoutConstraint.activate([
            searchBar.topAnchor.constraint(equalTo: topAnchor, constant: 20),
            searchBar.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            searchBar.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),

            sidebarContainer.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 12),
            sidebarContainer.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 20),
            sidebarContainer.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -20),
            sidebarContainer.widthAnchor.constraint(equalToConstant: 180),

            sidebarHeader.topAnchor.constraint(equalTo: sidebarContainer.topAnchor, constant: 4),
            sidebarHeader.leadingAnchor.constraint(equalTo: sidebarContainer.leadingAnchor, constant: 4),
            sidebarHeader.trailingAnchor.constraint(equalTo: sidebarSeparator.leadingAnchor, constant: -8),

            sidebarStack.topAnchor.constraint(equalTo: sidebarHeader.bottomAnchor, constant: 4),
            sidebarStack.leadingAnchor.constraint(equalTo: sidebarContainer.leadingAnchor),
            sidebarStack.trailingAnchor.constraint(equalTo: sidebarSeparator.leadingAnchor),
            sidebarStack.bottomAnchor.constraint(lessThanOrEqualTo: sidebarContainer.bottomAnchor),

            sidebarSeparator.topAnchor.constraint(equalTo: sidebarContainer.topAnchor),
            sidebarSeparator.trailingAnchor.constraint(equalTo: sidebarContainer.trailingAnchor),
            sidebarSeparator.bottomAnchor.constraint(equalTo: sidebarContainer.bottomAnchor),
            sidebarSeparator.widthAnchor.constraint(equalToConstant: 1),

            contentContainer.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 12),
            contentContainer.leadingAnchor.constraint(equalTo: sidebarContainer.trailingAnchor),
            contentContainer.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -20),
            contentContainer.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -20),

            featureGridView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            featureGridView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor, constant: 16),
            featureGridView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            featureGridView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            featureGridStack.topAnchor.constraint(equalTo: featureGridView.topAnchor),
            featureGridStack.leadingAnchor.constraint(equalTo: featureGridView.leadingAnchor),
            featureGridStack.trailingAnchor.constraint(lessThanOrEqualTo: featureGridView.trailingAnchor),

            resultsScrollView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            resultsScrollView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor, constant: 16),
            resultsScrollView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            resultsScrollView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            resultsStackView.topAnchor.constraint(equalTo: resultsScrollView.contentView.topAnchor),
            resultsStackView.leadingAnchor.constraint(equalTo: resultsScrollView.contentView.leadingAnchor),
            resultsStackView.trailingAnchor.constraint(equalTo: resultsScrollView.contentView.trailingAnchor),
            resultsStackView.widthAnchor.constraint(equalTo: resultsScrollView.contentView.widthAnchor),

            loadingLabel.centerXAnchor.constraint(equalTo: contentContainer.centerXAnchor),
            loadingLabel.centerYAnchor.constraint(equalTo: contentContainer.centerYAnchor)
        ])

        setupDetailContainer()
    }

    private func setupDetailContainer() {
        detailContainer.translatesAutoresizingMaskIntoConstraints = false
        detailContainer.alphaValue = 0
        detailContainer.isHidden = true

        detailTopBar.translatesAutoresizingMaskIntoConstraints = false
        detailTopBar.wantsLayer = true

        let backButton = NSButton(title: "", target: self, action: #selector(closeDetail))
        backButton.image = NSImage(systemSymbolName: "chevron.left", accessibilityDescription: nil)
        backButton.imagePosition = .imageLeading
        backButton.title = "Back"
        backButton.bezelStyle = .regularSquare
        backButton.isBordered = false
        backButton.font = .systemFont(ofSize: 13, weight: .medium)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        detailTitleLabel.font = .systemFont(ofSize: 15, weight: .semibold)
        detailTitleLabel.isEditable = false
        detailTitleLabel.isBordered = false
        detailTitleLabel.drawsBackground = false
        detailTitleLabel.alignment = .center
        detailTitleLabel.translatesAutoresizingMaskIntoConstraints = false

        let closeButton = NSButton(title: "", target: self, action: #selector(closeDetail))
        closeButton.image = NSImage(systemSymbolName: "xmark", accessibilityDescription: nil)
        closeButton.bezelStyle = .regularSquare
        closeButton.isBordered = false
        closeButton.translatesAutoresizingMaskIntoConstraints = false

        let separator = NSBox()
        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false

        detailTopBar.addSubview(backButton)
        detailTopBar.addSubview(detailTitleLabel)
        detailTopBar.addSubview(closeButton)
        detailTopBar.addSubview(separator)
        detailContainer.addSubview(detailTopBar)
        contentContainer.addSubview(detailContainer)

        NSLayoutConstraint.activate([
            detailContainer.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            detailContainer.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            detailContainer.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            detailContainer.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            detailTopBar.topAnchor.constraint(equalTo: detailContainer.topAnchor),
            detailTopBar.leadingAnchor.constraint(equalTo: detailContainer.leadingAnchor),
            detailTopBar.trailingAnchor.constraint(equalTo: detailContainer.trailingAnchor),
            detailTopBar.heightAnchor.constraint(equalToConstant: 40),

            backButton.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor, constant: 16),
            backButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            detailTitleLabel.centerXAnchor.constraint(equalTo: detailTopBar.centerXAnchor),
            detailTitleLabel.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            closeButton.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor, constant: -16),
            closeButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            separator.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: detailTopBar.bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 1)
        ])
    }

    func openModuleDetail(for card: FeatureCard, prefillTranslateText: String? = nil) {
        guard let detail = ModuleDetailRegistry.make(for: card.id) else {
            searchBar.stringValue = card.triggerKeyword
            searchBar.focus()
            handleTextChange(card.triggerKeyword)
            return
        }

        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = detail

        let contentView = detail.detailView
        contentView.translatesAutoresizingMaskIntoConstraints = false
        detailTopBar.isHidden = !detail.usesSharedTopBar
        detailContainer.addSubview(contentView)

        let contentTopAnchor = detail.usesSharedTopBar ? detailTopBar.bottomAnchor : detailContainer.topAnchor
        NSLayoutConstraint.activate([
            contentView.topAnchor.constraint(equalTo: contentTopAnchor),
            contentView.leadingAnchor.constraint(equalTo: detailContainer.leadingAnchor),
            contentView.trailingAnchor.constraint(equalTo: detailContainer.trailingAnchor),
            contentView.bottomAnchor.constraint(equalTo: detailContainer.bottomAnchor)
        ])
        detailTitleLabel.stringValue = detail.moduleTitle

        detailContainer.isHidden = false
        detailContainer.alphaValue = 0
        showingDetail = true

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.128
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            featureGridView.animator().alphaValue = 0
            resultsScrollView.animator().alphaValue = 0
            detailContainer.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                guard self.showingDetail else { return }
                self.featureGridView.isHidden = true
            }
        }

        detail.activate()

        if card.id == .translate, let text = prefillTranslateText ?? pendingTranslateText {
            pendingTranslateText = nil
            if let translate = detail as? TranslateDetailView {
                translate.prefill(text: text, autoTranslate: true)
            }
        }
    }

    func openTranslateDetail(with text: String) {
        guard let card = sortedCards.first(where: { $0.id == .translate }) else { return }
        pendingTranslateText = text
        if showingDetail, let translate = currentDetailObject as? TranslateDetailView {
            translate.prefill(text: text, autoTranslate: true)
            pendingTranslateText = nil
            return
        }
        openModuleDetail(for: card, prefillTranslateText: text)
    }

    func handleEscape() {
        if showingDetail {
            closeDetail()
        } else {
            onDismiss()
        }
    }

    @objc func closeDetail() {
        guard showingDetail else { return }
        showingDetail = false
        // Synchronously tear down the closing detail before any async work.
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        detailTopBar.isHidden = false

        featureGridView.isHidden = false
        featureGridView.alphaValue = 0

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.128
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            detailContainer.animator().alphaValue = 0
            featureGridView.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                // Stale: detail was reopened before close animation finished.
                guard !self.showingDetail else { return }
                self.detailContainer.isHidden = true
            }
        }
    }

    private func renderOpenApps() async {
        let selfPID = ProcessInfo.processInfo.processIdentifier
        let runningApps = NSWorkspace.shared.runningApplications.filter { app in
            app.activationPolicy == .regular
                && app.bundleIdentifier != nil
                && app.processIdentifier != selfPID
        }
        let bundleIDs = runningApps.compactMap(\.bundleIdentifier)
        let rankedIDs = await appActivationTracker.rankedBundleIDs(from: bundleIDs)
        var appsByBundleID: [String: NSRunningApplication] = [:]
        for app in runningApps {
            guard let bundleID = app.bundleIdentifier else { continue }
            appsByBundleID[bundleID] = appsByBundleID[bundleID] ?? app
        }
        let orderedApps = rankedIDs.prefix(10).compactMap { appsByBundleID[$0] }
        let frontmostBundleID = NSWorkspace.shared.frontmostApplication?.bundleIdentifier

        sidebarStack.arrangedSubviews.forEach { view in
            sidebarStack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        for app in orderedApps {
            let bundleID = app.bundleIdentifier
            let row = SidebarAppRow(
                app: app,
                isActive: bundleID == frontmostBundleID
            ) { [weak self] in
                self?.onActionDismiss()
                app.unhide()
                app.activate(from: NSRunningApplication.current, options: [.activateAllWindows])
            }
            sidebarStack.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: sidebarStack.widthAnchor).isActive = true
        }
    }

    private func renderFeatureCards() {
        featureGridStack.arrangedSubviews.forEach { view in
            featureGridStack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        widgetCards = []
        for (index, card) in sortedCards.enumerated() {
            let widget = WidgetFeatureCard(
                card: card,
                shortcutIndex: index + 1,
                statusSummary: statusSummary(for: card)
            ) { [weak self] selected in
                self?.onCardTapped(selected)
            }
            widgetCards.append(widget)
            featureGridStack.addArrangedSubview(widget)
        }
        refreshCardStatuses()
    }

    private func statusSummary(for card: FeatureCard) -> String {
        switch card.id {
        case .translate:
            let lang = languageDisplayName(for: TranslateDashboardStatus.targetLanguageCode)
            return "→ \(lang) · \(TranslateDashboardStatus.summary)"
        case .clipboard:
            return clipboardStatusSummary
        default:
            return card.subtitle
        }
    }

    private var clipboardStatusSummary = "Loading…"

    private func languageDisplayName(for code: String) -> String {
        switch code {
        case "en": return "English"
        case "zh-Hans": return "Chinese (Simplified)"
        case "zh-Hant": return "Chinese (Traditional)"
        case "ja": return "Japanese"
        case "ko": return "Korean"
        case "es": return "Spanish"
        case "fr": return "French"
        case "de": return "German"
        default: return code
        }
    }

    private func refreshCardStatuses() {
        Task { [weak self] in
            guard let self else { return }
            let clipStats: ClipboardStatistics
            if let module = ModuleDetailRegistry.clipboardModule {
                clipStats = await module.statistics()
            } else {
                clipStats = ClipboardStatistics(total: 0, pinned: 0)
            }
            let targetLang = await ModuleDetailRegistry.config?.translationTargetLanguage() ?? "en"
            await MainActor.run {
                self.clipboardStatusSummary = "\(clipStats.total) entries · \(clipStats.pinned) pinned"
                TranslateDashboardStatus.targetLanguageCode = targetLang
                for (index, card) in self.sortedCards.enumerated() where self.widgetCards.indices.contains(index) {
                    self.widgetCards[index].updateStatusSummary(self.statusSummary(for: card))
                }
            }
        }
    }

    private func onCardTapped(_ card: FeatureCard) {
        openModuleDetail(for: card)
    }

    private func crossfadeToResults(_ showResults: Bool) {
        if showResults {
            resultsScrollView.isHidden = false
            resultsScrollView.alphaValue = 0
            if showingDetail {
                contentContainer.addSubview(resultsScrollView, positioned: .above, relativeTo: detailContainer)
            }
        } else if !showingDetail {
            featureGridView.isHidden = false
            featureGridView.alphaValue = 0
        }
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.08
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            if showingDetail {
                resultsScrollView.animator().alphaValue = showResults ? 1 : 0
            } else {
                featureGridView.animator().alphaValue = showResults ? 0 : 1
                resultsScrollView.animator().alphaValue = showResults ? 1 : 0
            }
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                // Stale completion: state moved on while we were animating.
                guard self.showingResults == showResults else { return }
                if showResults {
                    if !self.showingDetail {
                        self.featureGridView.isHidden = true
                    }
                } else {
                    self.resultsScrollView.isHidden = true
                }
            }
        }
    }

    private func renderResults(_ items: [ResultItem]) {
        resultsStackView.arrangedSubviews.forEach { view in
            resultsStackView.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        currentItems = Array(items.prefix(6))
        selectedIndex = 0
        guard !currentItems.isEmpty else {
            if showingResults {
                crossfadeToResults(false)
                showingResults = false
            }
            return
        }
        if !showingResults {
            crossfadeToResults(true)
            showingResults = true
        }
        for (index, item) in currentItems.enumerated() {
            let row = WidgetResultRow(item: item, isSelected: index == selectedIndex) { [weak self] selected in
                self?.run(item: selected)
            }
            resultsStackView.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: resultsStackView.widthAnchor).isActive = true
        }
        resultsScrollView.contentView.scroll(to: NSPoint(x: 0, y: 0))
    }

    private func updateSelection(to newIndex: Int) {
        guard currentItems.indices.contains(newIndex) else { return }
        let oldIndex = selectedIndex
        selectedIndex = newIndex
        let rows = resultsStackView.arrangedSubviews.compactMap { $0 as? WidgetResultRow }
        if rows.indices.contains(oldIndex) {
            rows[oldIndex].setSelected(false)
        }
        if rows.indices.contains(newIndex) {
            rows[newIndex].setSelected(true)
        }
    }

    private func apply(snapshot: ResultSnapshot) {
        loadingLabel.isHidden = true
        let newItems = Array(snapshot.items.prefix(6))
        let newIDs = newItems.map(\.id)
        let currentIDs = currentItems.map(\.id)
        if newIDs != currentIDs {
            let previouslySelectedID: ResultID? = currentItems.indices.contains(selectedIndex)
                ? currentItems[selectedIndex].id
                : nil
            renderResults(newItems)
            if let previouslySelectedID,
               let preservedIndex = newItems.firstIndex(where: { $0.id == previouslySelectedID }),
               preservedIndex != selectedIndex {
                updateSelection(to: preservedIndex)
            }
        }
        if let paintMs = LatencyTracker.shared.markFirstPaint() {
            LatencyTelemetry.report(p95Milliseconds: paintMs)
        }
    }

    private func handleTextChange(_ text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            if showingResults {
                if showingDetail {
                    resultsScrollView.alphaValue = 0
                    resultsScrollView.isHidden = true
                } else {
                    crossfadeToResults(false)
                }
                showingResults = false
            }
            currentItems = []
            viewModel.cancel()
            return
        }
        if !modulesReady {
            loadingLabel.isHidden = false
            return
        }
        loadingLabel.isHidden = true
        if !showingResults {
            crossfadeToResults(true)
            showingResults = true
        }
        viewModel.queryChanged(text, issuedAt: .now)
    }

    @objc private func activateFirstSearchResult() {
        guard currentItems.indices.contains(selectedIndex) else { return }
        run(item: currentItems[selectedIndex])
    }

    private func run(item: ResultItem) {
        if case .translateText(let text) = item.primaryAction.kind {
            openTranslateDetail(with: text)
            return
        }
        onActionDismiss()
        Task {
            await actionExecutor.run(item.primaryAction, for: item)
        }
    }

    private func handleKeyCommand(_ command: LumaSearchBar.KeyCommand) -> Bool {
        switch command {
        case .down:
            guard !currentItems.isEmpty else { return true }
            updateSelection(to: min(selectedIndex + 1, currentItems.count - 1))
            return true
        case .up:
            guard !currentItems.isEmpty else { return true }
            updateSelection(to: max(selectedIndex - 1, 0))
            return true
        case .tab:
            guard currentItems.indices.contains(selectedIndex),
                  let action = currentItems[selectedIndex].secondaryActions.first else { return true }
            let item = currentItems[selectedIndex]
            onActionDismiss()
            Task { await actionExecutor.run(action, for: item) }
            return true
        case .commandNumber(let number):
            if !showingResults {
                if sortedCards.indices.contains(number - 1) {
                    onCardTapped(sortedCards[number - 1])
                }
                return true
            }
            let index = number - 1
            guard currentItems.indices.contains(index) else { return true }
            selectedIndex = index
            run(item: currentItems[index])
            return true
        }
    }
}
