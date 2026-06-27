import AppKit
import LumaCore

/// Home / search-results / module-detail transitions for the launcher main area.
@MainActor
final class LauncherContentCoordinator {
    private let listView: LauncherListView
    private let detailContainer: NSView
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
        detailContainer: NSView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        contentContainer: NSView
    ) {
        self.listView = listView
        self.detailContainer = detailContainer
        self.detailTopBar = detailTopBar
        self.detailTitleLabel = detailTitleLabel
        self.contentContainer = contentContainer
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

        detailContainer.isHidden = false
        detailContainer.alphaValue = 0
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
    }

    func closeDetail() {
        guard showingDetail else { return }
        showingDetail = false
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        currentDetailModuleID = nil
        detailTopBar.isHidden = false

        listView.isHidden = false
        listView.alphaValue = 0

        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = MotionTokens.panelShowDuration
            ctx.timingFunction = CAMediaTimingFunction(name: .easeOut)
            detailContainer.animator().alphaValue = 0
            listView.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor [weak self] in
                guard let self, !self.showingDetail else { return }
                self.detailContainer.isHidden = true
            }
        }
        onHomeSessionSaved?()
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
            return
        }
        showingResults = true
        listView.renderResults(newItems, layout: layout, preserveSelectionID: previouslySelectedID)
        syncSelectionFromList()
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
        guard showingResults else { return }
        if showingDetail {
            showingResults = false
            currentItems = []
            selectedIndex = 0
        } else {
            showingResults = false
            currentItems = []
            selectedIndex = 0
        }
    }

    func beginShowingResults() {
        guard !showingResults else { return }
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
