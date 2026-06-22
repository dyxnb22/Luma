import AppKit
import LumaCore

@MainActor
final class LauncherRootView: NSView {
    private let glassBackground = NSVisualEffectView()
    private let highlightLayer = CAGradientLayer()
    private let searchBar = LumaSearchBar()
    private let sidebarContainer = NSView()
    private let sidebarStack = NSStackView()
    private let contentContainer = NSView()
    private let featureGridView = NSView()
    private let resultsScrollView = NSScrollView()
    private let resultsStackView = NSStackView()
    private let featureGridStack = NSStackView()
    private let loadingLabel = NSTextField(labelWithString: "Loading…")
    private let cards: [FeatureCard]
    private let viewModel: LauncherViewModel
    private let actionExecutor: ActionExecutor
    private let appActivationTracker: AppActivationTracker
    private let onDismiss: () -> Void
    private var currentItems: [ResultItem] = []
    private var selectedIndex = 0
    private var modulesReady = false
    private var showingResults = false

    init(
        cards: [FeatureCard],
        viewModel: LauncherViewModel,
        actionExecutor: ActionExecutor,
        appActivationTracker: AppActivationTracker,
        onDismiss: @escaping () -> Void
    ) {
        self.cards = cards
        self.viewModel = viewModel
        self.actionExecutor = actionExecutor
        self.appActivationTracker = appActivationTracker
        self.onDismiss = onDismiss
        super.init(frame: .zero)

        searchBar.onEscape = onDismiss
        searchBar.onReturn = { [weak self] in self?.activateFirstSearchResult() }
        searchBar.onTextChange = { [weak self] text in self?.handleTextChange(text) }
        searchBar.onKeyCommand = { [weak self] command in self?.handleKeyCommand(command) ?? false }

        viewModel.onSnapshot = { [weak self] snapshot in
            self?.apply(snapshot: snapshot)
        }

        setupGlassChrome()
        setupLayout()
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

    func showHome() {
        searchBar.stringValue = ""
        showingResults = false
        renderResults([])
        featureGridView.isHidden = false
        featureGridView.alphaValue = 1
        resultsScrollView.isHidden = true
        resultsScrollView.alphaValue = 0
        renderFeatureCards()
        loadingLabel.isHidden = modulesReady
        refreshOpenApps()
        focusSearchField()
    }

    func refreshOpenApps() {
        Task { await renderOpenApps() }
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
        sidebarStack.alignment = .width
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
        featureGridStack.alignment = .top
        featureGridStack.translatesAutoresizingMaskIntoConstraints = false

        featureGridView.translatesAutoresizingMaskIntoConstraints = false
        featureGridView.addSubview(featureGridStack)

        resultsStackView.orientation = .vertical
        resultsStackView.spacing = 6
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

            sidebarStack.topAnchor.constraint(equalTo: sidebarHeader.bottomAnchor, constant: 8),
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
        let appsByBundleID = Dictionary(uniqueKeysWithValues: runningApps.compactMap { app -> (String, NSRunningApplication)? in
            guard let bundleID = app.bundleIdentifier else { return nil }
            return (bundleID, app)
        })
        let orderedApps = rankedIDs.prefix(10).compactMap { appsByBundleID[$0] }
        let frontmostBundleID = NSWorkspace.shared.frontmostApplication?.bundleIdentifier

        sidebarStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for app in orderedApps {
            let bundleID = app.bundleIdentifier
            let row = SidebarAppRow(
                app: app,
                isActive: bundleID == frontmostBundleID
            ) {
                app.activate()
            }
            sidebarStack.addArrangedSubview(row)
        }
    }

    private func renderFeatureCards() {
        featureGridStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let sorted = cards.sorted { $0.position.column < $1.position.column }
        for (index, card) in sorted.prefix(4).enumerated() {
            featureGridStack.addArrangedSubview(WidgetFeatureCard(card: card, shortcutIndex: index + 1) { [weak self] selected in
                self?.onCardTapped(selected)
            })
        }
    }

    private func onCardTapped(_ card: FeatureCard) {
        searchBar.stringValue = card.triggerKeyword
        searchBar.focus()
        handleTextChange(card.triggerKeyword)
    }

    private func crossfadeToResults(_ showResults: Bool) {
        if showResults {
            resultsScrollView.isHidden = false
            resultsScrollView.alphaValue = 0
        } else {
            featureGridView.isHidden = false
            featureGridView.alphaValue = 0
        }
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.08
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            featureGridView.animator().alphaValue = showResults ? 0 : 1
            resultsScrollView.animator().alphaValue = showResults ? 1 : 0
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self else { return }
                if showResults {
                    self.featureGridView.isHidden = true
                } else {
                    self.resultsScrollView.isHidden = true
                }
            }
        }
    }

    private func renderResults(_ items: [ResultItem]) {
        resultsStackView.arrangedSubviews.forEach { $0.removeFromSuperview() }
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
            resultsStackView.addArrangedSubview(WidgetResultRow(item: item, isSelected: index == selectedIndex) { [weak self] selected in
                self?.run(item: selected)
            })
        }
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
            renderResults(newItems)
        }
        #if DEBUG
        if let p95 = viewModel.p95LatencyMilliseconds(for: snapshot.querySequence) {
            LatencyTelemetry.report(p95Milliseconds: p95)
        }
        #endif
    }

    private func handleTextChange(_ text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            if showingResults {
                crossfadeToResults(false)
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
        onDismiss()
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
            onDismiss()
            Task { await actionExecutor.run(action, for: item) }
            return true
        case .commandNumber(let number):
            if !showingResults {
                let sorted = cards.sorted { $0.position.column < $1.position.column }
                if sorted.indices.contains(number - 1) {
                    onCardTapped(sorted[number - 1])
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
