import AppKit
import LumaCore
import LumaModules

@MainActor
final class LauncherRootView: NSVisualEffectView {
    private let searchField = LumaSearchField()
    private let sidebarScroll = NSScrollView()
    private let sidebarStack = NSStackView()
    private let contentStack = NSStackView()
    private let recentStack = NSStackView()
    private let appResultsStack = NSStackView()
    private let featureGrid = NSGridView()
    private let latencyHUD = LatencyHUD()
    private let cards: [FeatureCard]
    private let cardLayoutStore: CardLayoutStore
    private let viewModel: LauncherViewModel
    private let actionExecutor: ActionExecutor
    private let onDismiss: () -> Void
    private let onOpenFeature: () -> Void
    private var currentItems: [ResultItem] = []
    private var selectedIndex = 0

    init(cards: [FeatureCard], cardLayoutStore: CardLayoutStore, viewModel: LauncherViewModel, actionExecutor: ActionExecutor, onDismiss: @escaping () -> Void, onOpenFeature: @escaping () -> Void, onBackOrDismiss: @escaping () -> Void) {
        self.cards = cards
        self.cardLayoutStore = cardLayoutStore
        self.viewModel = viewModel
        self.actionExecutor = actionExecutor
        self.onDismiss = onDismiss
        self.onOpenFeature = onOpenFeature
        super.init(frame: .zero)
        material = .hudWindow
        blendingMode = .behindWindow
        state = .active
        wantsLayer = true
        layer?.cornerRadius = 28
        layer?.cornerCurve = .continuous
        layer?.masksToBounds = true
        searchField.onEscape = onBackOrDismiss
        searchField.onKeyCommand = { [weak self] command in
            self?.handleKeyCommand(command) ?? false
        }
        viewModel.onSnapshot = { [weak self] snapshot in
            self?.apply(snapshot: snapshot)
        }
        setupLayout()
        showHome()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func focusSearchField() {
        window?.makeFirstResponder(searchField)
    }

    func showHome() {
        searchField.stringValue = ""
        renderRunningApps()
        renderRecentItems()
        renderResults([])
        recentStack.isHidden = false
        featureGrid.isHidden = false
        renderFeatureGrid(cards)
        focusSearchField()
    }

    private func renderRecentItems() {
        recentStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        Task {
            let results = await viewModel.recentFrecency(limit: 8)
            guard searchField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
            let items = RecentItemProvider.recentItems(from: results) { [weak self] item in
                self?.run(item: item)
            }
            guard !items.isEmpty else {
                recentStack.isHidden = true
                return
            }
            recentStack.isHidden = false
            for item in items {
                recentStack.addArrangedSubview(RecentItemButton(item: item))
            }
        }
    }

    private func setupLayout() {
        let root = NSStackView()
        root.orientation = .horizontal
        root.spacing = 18
        root.edgeInsets = NSEdgeInsets(top: 22, left: 22, bottom: 22, right: 22)
        root.translatesAutoresizingMaskIntoConstraints = false
        addSubview(root)

        sidebarScroll.hasVerticalScroller = true
        sidebarScroll.scrollerStyle = .overlay
        sidebarScroll.autohidesScrollers = true
        sidebarScroll.drawsBackground = false
        sidebarScroll.translatesAutoresizingMaskIntoConstraints = false
        sidebarScroll.widthAnchor.constraint(equalToConstant: 172).isActive = true

        sidebarStack.orientation = .vertical
        sidebarStack.alignment = .leading
        sidebarStack.spacing = 8
        sidebarStack.edgeInsets = NSEdgeInsets(top: 4, left: 4, bottom: 4, right: 8)
        sidebarScroll.documentView = sidebarStack

        contentStack.orientation = .vertical
        contentStack.spacing = 18
        contentStack.translatesAutoresizingMaskIntoConstraints = false

        setupSearchField()
        setupRecentItems()
        setupAppResults()
        setupFeatureGrid()
        setupLatencyHUD()

        root.addArrangedSubview(sidebarScroll)
        root.addArrangedSubview(contentStack)

        NSLayoutConstraint.activate([
            root.topAnchor.constraint(equalTo: topAnchor),
            root.leadingAnchor.constraint(equalTo: leadingAnchor),
            root.trailingAnchor.constraint(equalTo: trailingAnchor),
            root.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    private func setupSearchField() {
        searchField.placeholderString = "Search apps, translate, clipboard, notes..."
        searchField.font = .systemFont(ofSize: 24, weight: .regular)
        searchField.controlSize = .large
        searchField.delegate = self
        searchField.target = self
        searchField.action = #selector(activateFirstSearchResult)
        searchField.translatesAutoresizingMaskIntoConstraints = false
        searchField.heightAnchor.constraint(equalToConstant: 46).isActive = true
        contentStack.addArrangedSubview(searchField)
    }

    private func setupAppResults() {
        appResultsStack.orientation = .vertical
        appResultsStack.spacing = 8
        contentStack.addArrangedSubview(appResultsStack)
    }

    private func setupRecentItems() {
        recentStack.orientation = .horizontal
        recentStack.spacing = 10
        contentStack.addArrangedSubview(recentStack)
    }

    private func setupFeatureGrid() {
        featureGrid.xPlacement = .fill
        featureGrid.yPlacement = .fill
        featureGrid.rowSpacing = 20
        featureGrid.columnSpacing = 24
        contentStack.addArrangedSubview(featureGrid)
    }

    private func setupLatencyHUD() {
        addSubview(latencyHUD)
        NSLayoutConstraint.activate([
            latencyHUD.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -22),
            latencyHUD.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -14),
            latencyHUD.widthAnchor.constraint(equalToConstant: 88),
            latencyHUD.heightAnchor.constraint(equalToConstant: 24)
        ])
    }

    private func renderRunningApps() {
        sidebarStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        sidebarStack.addArrangedSubview(SidebarHeaderView(title: "Open Apps"))

        let apps = NSWorkspace.shared.runningApplications
            .filter { $0.activationPolicy == .regular && $0.localizedName != nil }
            .sorted {
                if $0.isActive != $1.isActive { return $0.isActive }
                return ($0.launchDate ?? .distantPast) > ($1.launchDate ?? .distantPast)
            }

        for app in apps {
            sidebarStack.addArrangedSubview(RunningAppRow(app: app))
        }
    }

    private func renderResults(_ items: [ResultItem]) {
        appResultsStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        currentItems = Array(items.prefix(12))
        selectedIndex = 0
        guard !currentItems.isEmpty else {
            appResultsStack.isHidden = true
            return
        }
        appResultsStack.isHidden = false
        for (index, item) in currentItems.enumerated() {
            appResultsStack.addArrangedSubview(ResultItemRow(item: item, isSelected: index == selectedIndex) { [weak self] selected in
                self?.run(item: selected)
            })
        }
    }

    private func updateSelection(to newIndex: Int) {
        guard currentItems.indices.contains(newIndex) else { return }
        let oldIndex = selectedIndex
        selectedIndex = newIndex
        let rows = appResultsStack.arrangedSubviews.compactMap { $0 as? ResultItemRow }
        if rows.indices.contains(oldIndex) {
            rows[oldIndex].setSelected(false)
        }
        if rows.indices.contains(newIndex) {
            rows[newIndex].setSelected(true)
        }
    }

    private func apply(snapshot: ResultSnapshot) {
        let newItems = Array(snapshot.items.prefix(12))
        let newIDs = newItems.map(\.id)
        let currentIDs = currentItems.map(\.id)
        if newIDs != currentIDs {
            renderResults(newItems)
        }
        if let p95 = viewModel.p95LatencyMilliseconds(for: snapshot.querySequence) {
            latencyHUD.update(p95Milliseconds: p95)
        }
    }

    private func renderFeatureGrid(_ cards: [FeatureCard]) {
        while featureGrid.numberOfRows > 0 {
            featureGrid.removeRow(at: 0)
        }

        let sorted = cards.sorted {
            if $0.position.row == $1.position.row {
                return $0.position.column < $1.position.column
            }
            return $0.position.row < $1.position.row
        }

        for rowStart in stride(from: 0, to: sorted.count, by: 4) {
            let rowCards = sorted[rowStart..<min(rowStart + 4, sorted.count)]
            let views = rowCards.map { FeatureIconView(card: $0, cardLayoutStore: cardLayoutStore, onOpen: onOpenFeature) }
            featureGrid.addRow(with: views)
        }
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

    private func handleKeyCommand(_ command: LumaSearchField.KeyCommand) -> Bool {
        switch command {
        case .down:
            guard !currentItems.isEmpty else { return true }
            updateSelection(to: min(selectedIndex + 1, currentItems.count - 1))
            return true
        case .up:
            guard !currentItems.isEmpty else { return true }
            updateSelection(to: max(selectedIndex - 1, 0))
            return true
        case .commandNumber(let number):
            let index = number - 1
            guard currentItems.indices.contains(index) else { return true }
            selectedIndex = index
            run(item: currentItems[index])
            return true
        case .secondary:
            guard currentItems.indices.contains(selectedIndex),
                  let action = currentItems[selectedIndex].secondaryActions.first else { return true }
            let item = currentItems[selectedIndex]
            onDismiss()
            Task { await actionExecutor.run(action, for: item) }
            return true
        }
    }
}

extension LauncherRootView: NSSearchFieldDelegate {
    func controlTextDidChange(_ obj: Notification) {
        let text = searchField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else {
            appResultsStack.isHidden = true
            recentStack.isHidden = false
            featureGrid.isHidden = false
            currentItems = []
            viewModel.cancel()
            latencyHUD.update(p95Milliseconds: nil)
            return
        }
        recentStack.isHidden = true
        featureGrid.isHidden = true
        viewModel.queryChanged(text, issuedAt: .now)
    }
}

@MainActor
final class LumaSearchField: NSSearchField {
    enum KeyCommand {
        case up
        case down
        case commandNumber(Int)
        case secondary
    }

    var onEscape: (() -> Void)?
    var onKeyCommand: ((KeyCommand) -> Bool)?

    override func cancelOperation(_ sender: Any?) {
        onEscape?()
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 125, onKeyCommand?(.down) == true { return }
        if event.keyCode == 126, onKeyCommand?(.up) == true { return }
        if event.keyCode == 48, onKeyCommand?(.secondary) == true { return }
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers,
           let number = Int(chars),
           (1...9).contains(number),
           onKeyCommand?(.commandNumber(number)) == true {
            return
        }
        super.keyDown(with: event)
    }
}

@MainActor
final class SidebarHeaderView: NSTextField {
    init(title: String) {
        super.init(frame: .zero)
        stringValue = title
        isEditable = false
        isBordered = false
        drawsBackground = false
        textColor = .secondaryLabelColor
        font = .systemFont(ofSize: 12, weight: .semibold)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

@MainActor
final class RunningAppRow: NSView {
    init(app: NSRunningApplication) {
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 12
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = app.isActive
            ? NSColor.controlAccentColor.withAlphaComponent(0.18).cgColor
            : NSColor.clear.cgColor

        let icon = NSImageView(image: IconCache.shared.runningAppIcon(app))
        icon.imageScaling = .scaleProportionallyUpOrDown
        icon.translatesAutoresizingMaskIntoConstraints = false

        let label = NSTextField(labelWithString: app.localizedName ?? "App")
        label.font = .systemFont(ofSize: 13, weight: .medium)
        label.lineBreakMode = .byTruncatingTail

        let row = NSStackView(views: [icon, label])
        row.orientation = .horizontal
        row.alignment = .centerY
        row.spacing = 8
        row.translatesAutoresizingMaskIntoConstraints = false
        addSubview(row)

        NSLayoutConstraint.activate([
            widthAnchor.constraint(equalToConstant: 156),
            heightAnchor.constraint(equalToConstant: 38),
            icon.widthAnchor.constraint(equalToConstant: 24),
            icon.heightAnchor.constraint(equalToConstant: 24),
            row.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            row.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            row.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

@MainActor
final class ResultItemRow: NSButton {
    private let item: ResultItem
    private let onRun: (ResultItem) -> Void

    init(item: ResultItem, isSelected: Bool, onRun: @escaping (ResultItem) -> Void) {
        self.item = item
        self.onRun = onRun
        super.init(frame: .zero)
        title = item.title
        image = Self.iconImage(for: item.icon)
        imagePosition = .imageLeading
        alignment = .left
        bezelStyle = .regularSquare
        isBordered = false
        wantsLayer = true
        layer?.cornerRadius = 14
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = (isSelected ? NSColor.controlAccentColor.withAlphaComponent(0.28) : NSColor.controlBackgroundColor.withAlphaComponent(0.58)).cgColor
        target = self
        action = #selector(run)
        heightAnchor.constraint(equalToConstant: 44).isActive = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func setSelected(_ isSelected: Bool) {
        layer?.backgroundColor = (isSelected
            ? NSColor.controlAccentColor.withAlphaComponent(0.28)
            : NSColor.controlBackgroundColor.withAlphaComponent(0.58)).cgColor
    }

    @objc private func run() {
        onRun(item)
    }

    private static func iconImage(for icon: LumaCore.IconRef) -> NSImage? {
        switch icon {
        case .bundleID(let bundleID):
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) {
                return IconCache.shared.appIcon(for: url)
            }
            return NSImage(systemSymbolName: "app.fill", accessibilityDescription: nil)
        case .symbol(let symbol):
            return NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        case .file(let url):
            return IconCache.shared.appIcon(for: url)
        case .none:
            return NSImage(systemSymbolName: "sparkles", accessibilityDescription: nil)
        }
    }
}

@MainActor
final class FeatureIconView: NSButton {
    private var dragStart: NSPoint?
    private let cardID: ModuleIdentifier
    private let cardLayoutStore: CardLayoutStore
    private let onOpen: () -> Void

    init(card: FeatureCard, cardLayoutStore: CardLayoutStore, onOpen: @escaping () -> Void) {
        self.cardID = card.id
        self.cardLayoutStore = cardLayoutStore
        self.onOpen = onOpen
        super.init(frame: .zero)
        title = card.title
        image = Self.image(for: card.id)
        imagePosition = .imageAbove
        alignment = .center
        font = .systemFont(ofSize: 13, weight: .medium)
        isBordered = false
        wantsLayer = true
        layer?.cornerRadius = 22
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.45).cgColor
        target = self
        action = #selector(open)

        NSLayoutConstraint.activate([
            widthAnchor.constraint(equalToConstant: 150),
            heightAnchor.constraint(equalToConstant: 132)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func mouseDown(with event: NSEvent) {
        dragStart = convert(event.locationInWindow, from: nil)
        super.mouseDown(with: event)
    }

    override func mouseDragged(with event: NSEvent) {
        guard let dragStart else { return }
        let current = convert(event.locationInWindow, from: nil)
        var next = frame
        next.origin.x += current.x - dragStart.x
        next.origin.y += current.y - dragStart.y
        if let superview {
            next.origin.x = min(max(0, next.origin.x), max(0, superview.bounds.width - next.width))
            next.origin.y = min(max(0, next.origin.y), max(0, superview.bounds.height - next.height))
        }
        frame = next
    }

    override func mouseUp(with event: NSEvent) {
        let columnWidth = 174
        let rowHeight = 152
        let position = CardPosition(
            column: max(0, Int((frame.midX / CGFloat(columnWidth)).rounded())),
            row: max(0, Int((frame.midY / CGFloat(rowHeight)).rounded())),
            zIndex: Int(Date().timeIntervalSince1970)
        )
        try? cardLayoutStore.save(position: position, for: cardID)
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.14
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            animator().frame.origin = NSPoint(x: CGFloat(position.column * columnWidth), y: CGFloat(position.row * rowHeight))
        }
        dragStart = nil
        super.mouseUp(with: event)
    }

    @objc private func open() {
        onOpen()
    }

    private static func image(for id: ModuleIdentifier) -> NSImage? {
        let symbol: String
        switch id.rawValue {
        case "luma.translate": symbol = "character.bubble.fill"
        case "luma.clipboard": symbol = "doc.on.clipboard.fill"
        case "luma.secrets": symbol = "lock.shield.fill"
        case "luma.window-layouts": symbol = "rectangle.split.2x1.fill"
        case "luma.notes": symbol = "note.text"
        case "luma.wordbook": symbol = "text.book.closed.fill"
        default: symbol = "sparkles"
        }
        let config = NSImage.SymbolConfiguration(pointSize: 42, weight: .regular)
        return NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?.withSymbolConfiguration(config)
    }
}
