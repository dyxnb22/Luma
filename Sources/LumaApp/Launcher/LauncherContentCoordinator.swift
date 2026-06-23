import AppKit
import LumaCore
import LumaModules

/// Home / search-results / module-detail transitions for the launcher main area.
@MainActor
final class LauncherContentCoordinator {
    private let homeScrollView: NSScrollView
    private let resultsScrollView: NSScrollView
    private let detailContainer: NSView
    private let detailTopBar: NSView
    private let detailTitleLabel: NSTextField
    private let contentContainer: NSView
    private let resultsStackView: FlippedStackView

    private(set) var showingDetail = false
    private(set) var showingResults = false
    private(set) var currentItems: [ResultItem] = []
    private(set) var selectedIndex = 0
    private(set) var currentDetailObject: (any ModuleDetailView)?
    private(set) var currentDetailModuleID: ModuleIdentifier?
    var pendingTranslateText: String?

    var onSessionChanged: (() -> Void)?
    var onHomeSessionSaved: (() -> Void)?

    init(
        homeScrollView: NSScrollView,
        resultsScrollView: NSScrollView,
        detailContainer: NSView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        contentContainer: NSView,
        resultsStackView: FlippedStackView
    ) {
        self.homeScrollView = homeScrollView
        self.resultsScrollView = resultsScrollView
        self.detailContainer = detailContainer
        self.detailTopBar = detailTopBar
        self.detailTitleLabel = detailTitleLabel
        self.contentContainer = contentContainer
        self.resultsStackView = resultsStackView
    }

    func tearDownDetailIfNeeded() {
        guard showingDetail else { return }
        showingDetail = false
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        currentDetailModuleID = nil
        detailContainer.isHidden = true
        detailContainer.alphaValue = 0
    }

    func resetResults() {
        showingResults = false
        currentItems = []
        selectedIndex = 0
        renderResults([], onRun: { _ in })
    }

    func present(
        _ detail: any ModuleDetailView,
        moduleID: ModuleIdentifier,
        card: FeatureCard? = nil,
        prefillTranslateText: String? = nil
    ) {
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = detail
        currentDetailModuleID = moduleID

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
            homeScrollView.animator().alphaValue = 0
            resultsScrollView.animator().alphaValue = 0
            detailContainer.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, self.showingDetail else { return }
                self.homeScrollView.isHidden = true
            }
        }

        detail.activate()

        if let card, card.id == .translate, let text = prefillTranslateText ?? pendingTranslateText {
            pendingTranslateText = nil
            if let translate = detail as? TranslateDetailView {
                translate.prefill(text: text, autoTranslate: true)
            }
        }
        onSessionChanged?()
    }

    func closeDetail() {
        guard showingDetail else { return }
        showingDetail = false
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        currentDetailModuleID = nil
        detailTopBar.isHidden = false

        homeScrollView.isHidden = false
        homeScrollView.alphaValue = 0

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.128
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            detailContainer.animator().alphaValue = 0
            homeScrollView.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, !self.showingDetail else { return }
                self.detailContainer.isHidden = true
            }
        }
        onHomeSessionSaved?()
    }

    func crossfadeToResults(_ showResults: Bool) {
        if showResults {
            resultsScrollView.isHidden = false
            resultsScrollView.alphaValue = 0
            if showingDetail {
                contentContainer.addSubview(resultsScrollView, positioned: .above, relativeTo: detailContainer)
            }
        } else if !showingDetail {
            homeScrollView.isHidden = false
            homeScrollView.alphaValue = 0
        }
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.08
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            if showingDetail {
                resultsScrollView.animator().alphaValue = showResults ? 1 : 0
            } else {
                homeScrollView.animator().alphaValue = showResults ? 0 : 1
                resultsScrollView.animator().alphaValue = showResults ? 1 : 0
            }
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, self.showingResults == showResults else { return }
                if showResults {
                    if !self.showingDetail {
                        self.homeScrollView.isHidden = true
                    }
                } else {
                    self.resultsScrollView.isHidden = true
                }
            }
        }
    }

    func renderResults(_ items: [ResultItem], onRun: @escaping (ResultItem) -> Void) {
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
            let row = WidgetResultRow(item: item, isSelected: index == selectedIndex) { selected in
                onRun(selected)
            }
            resultsStackView.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: resultsStackView.widthAnchor).isActive = true
        }
        resultsScrollView.contentView.scroll(to: NSPoint(x: 0, y: 0))
    }

    func updateSelection(to newIndex: Int) {
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

    func apply(snapshot: ResultSnapshot, onRun: @escaping (ResultItem) -> Void) {
        let newItems = Array(snapshot.items.prefix(6))
        let newIDs = newItems.map(\.id)
        let currentIDs = currentItems.map(\.id)
        guard newIDs != currentIDs else { return }
        let previouslySelectedID: ResultID? = currentItems.indices.contains(selectedIndex)
            ? currentItems[selectedIndex].id
            : nil
        renderResults(newItems, onRun: onRun)
        if let previouslySelectedID,
           let preservedIndex = newItems.firstIndex(where: { $0.id == previouslySelectedID }),
           preservedIndex != selectedIndex {
            updateSelection(to: preservedIndex)
        }
    }

    func hideResultsOverlay() {
        resultsScrollView.alphaValue = 0
        resultsScrollView.isHidden = true
        showingResults = false
        currentItems = []
    }

    func beginShowingResults() {
        guard !showingResults else { return }
        crossfadeToResults(true)
        showingResults = true
    }

    func dismissResultsForEmptyQuery() {
        guard showingResults else { return }
        if showingDetail {
            hideResultsOverlay()
        } else {
            crossfadeToResults(false)
            showingResults = false
            currentItems = []
        }
    }
}
