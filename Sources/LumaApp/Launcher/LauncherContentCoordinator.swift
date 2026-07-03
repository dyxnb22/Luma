import AppKit
import LumaCore

/// Home / search-results / module-detail transitions for the launcher main area.
@MainActor
final class LauncherContentCoordinator {
    private let listView: LauncherListView
    private let detailContainer: LauncherOverlayHostView
    private let detailTopBar: NSView
    private let detailTitleLabel: NSTextField
    private let contentContainer: NSView

    private(set) var showingDetail = false
    private(set) var showingResults = false
    private(set) var currentItems: [ResultItem] = []
    private(set) var selectedIndex = 0
    private(set) var currentDetailObject: (any ModuleDetailView)?
    private(set) var currentDetailModuleID: ModuleIdentifier?
    var pendingTranslateText: String?

    var onSessionChanged: (() -> Void)?
    var onHomeSessionSaved: (() -> Void)?
    var onRun: ((ResultItem) -> Void)?
    var onRightClick: ((ResultItem) -> Void)?
    var onSelectionChanged: (() -> Void)?

    init(
        listView: LauncherListView,
        detailContainer: LauncherOverlayHostView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        contentContainer: NSView
    ) {
        self.listView = listView
        self.detailContainer = detailContainer
        self.detailTopBar = detailTopBar
        self.detailTitleLabel = detailTitleLabel
        self.contentContainer = contentContainer
        detailContainer.passesHitTests = false
        detailContainer.isHidden = true
        listView.onRun = { [weak self] item in self?.onRun?(item) }
        listView.onRightClick = { [weak self] item in self?.onRightClick?(item) }
        listView.onSelectionChanged = { [weak self] index in
            guard let self else { return }
            self.selectedIndex = index
            self.currentItems = self.listView.currentItems
            self.onSelectionChanged?()
        }
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
        detailContainer.passesHitTests = false
        listView.passesHitTests = true
        onHomeSessionSaved?()
    }

    func resetResults() {
        showingResults = false
        currentItems = []
        selectedIndex = 0
        listView.clear()
    }

    func showHome(_ snapshot: LauncherHomeSnapshot) {
        showingResults = false
        currentItems = snapshot.flatItems
        selectedIndex = 0
        listView.renderHome(snapshot)
        syncSelectionFromList()
        LauncherInPanelLayout.stabilizePanel(from: listView)
    }

    func present(
        _ detail: any ModuleDetailView,
        moduleID: ModuleIdentifier,
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

        listView.passesHitTests = false
        detailContainer.isHidden = false
        detailContainer.alphaValue = 0
        detailContainer.passesHitTests = true
        showingDetail = true

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = MotionTokens.panelShowDuration
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            listView.animator().alphaValue = 0
            detailContainer.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, self.showingDetail else { return }
                self.listView.isHidden = true
                self.listView.passesHitTests = false
                LauncherInPanelLayout.stabilizePanel(from: self.detailContainer)
            }
        }

        detail.activate()

        if moduleID == .translate, let text = prefillTranslateText ?? pendingTranslateText {
            pendingTranslateText = nil
            if let translate = detail as? TranslateDetailView {
                translate.prefill(text: text, autoTranslate: true)
            }
        }
        onSessionChanged?()
        LauncherInPanelLayout.stabilizePanel(from: detailContainer)
    }

    func closeDetail() {
        guard showingDetail else { return }
        showingDetail = false
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        currentDetailModuleID = nil
        detailTopBar.isHidden = false

        detailContainer.passesHitTests = false
        listView.isHidden = false
        listView.alphaValue = 0
        listView.passesHitTests = true

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = MotionTokens.panelShowDuration
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            detailContainer.animator().alphaValue = 0
            listView.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, !self.showingDetail else { return }
                self.detailContainer.isHidden = true
                self.detailContainer.passesHitTests = false
                self.listView.passesHitTests = true
                LauncherInPanelLayout.stabilizePanel(from: self.listView)
            }
        }
        onHomeSessionSaved?()
        LauncherInPanelLayout.stabilizePanel(from: detailContainer)
    }

    func renderResults(_ items: [ResultItem], layout: ResultListLayout = .flat) {
        let newItems = Array(items.prefix(8))
        let previouslySelectedID: ResultID? = currentItems[safe: selectedIndex]?.id
        currentItems = newItems
        guard !newItems.isEmpty else {
            showingResults = true
            currentItems = []
            selectedIndex = 0
            listView.renderResults([], layout: layout)
            LauncherInPanelLayout.stabilizePanel(from: listView)
            return
        }
        showingResults = true
        listView.renderResults(newItems, layout: layout, preserveSelectionID: previouslySelectedID)
        syncSelectionFromList()
        LauncherInPanelLayout.stabilizePanel(from: listView)
    }

    func apply(snapshot: ResultSnapshot) {
        let newItems = Array(snapshot.items.prefix(8))
        let newIDs = newItems.map(\.id)
        let currentIDs = currentItems.map(\.id)
        let currentLayout = listView.currentLayout
        guard newIDs != currentIDs || snapshot.layout != currentLayout else { return }
        renderResults(newItems, layout: snapshot.layout)
    }

    func updateSelection(to newIndex: Int) {
        guard currentItems.indices.contains(newIndex) else { return }
        selectedIndex = newIndex
        listView.updateSelection(to: newIndex)
    }

    func dismissResultsForEmptyQuery() {
        showingResults = false
        currentItems = []
        selectedIndex = 0
        listView.clear()
    }

    func beginShowingResults(clearStaleContent: Bool = false) {
        if clearStaleContent || !showingResults {
            listView.clear()
            currentItems = []
            selectedIndex = 0
        }
        showingResults = true
    }

    private func syncSelectionFromList() {
        selectedIndex = listView.selectedFlatIndex
        currentItems = listView.currentItems
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
