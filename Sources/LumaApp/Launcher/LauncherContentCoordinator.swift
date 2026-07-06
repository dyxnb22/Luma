import AppKit
import LumaCore

/// Module detail opens in the launcher's right column; Open Apps stay in the left column (ADR-032).
enum ModuleDetailPresentation: Equatable {
    case rightColumn
}

/// Home / search-results / module-detail transitions for the launcher main area.
@MainActor
final class LauncherContentCoordinator {
    private let listView: LauncherListView
    private let detailContainer: LauncherOverlayHostView
    private let detailTopBar: NSView
    private let detailTitleLabel: NSTextField
    private let contentContainer: NSView

    private(set) var mode: LauncherContentMode = .home
    private(set) var currentItems: [ResultItem] = []
    private(set) var selectedIndex = 0
    private(set) var currentDetailObject: (any ModuleDetailView)?
    private var detailModuleIDStorage: ModuleIdentifier?
    var showingDetail: Bool { mode.showingDetail }
    var showingResults: Bool { mode.showingResults }
    var currentDetailModuleID: ModuleIdentifier? { detailModuleIDStorage ?? mode.detailModuleID }
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
        mode = .home
        currentDetailObject?.deactivate()
        currentDetailObject?.detailView.removeFromSuperview()
        currentDetailObject = nil
        detailModuleIDStorage = nil
        detailContainer.isHidden = true
        detailContainer.alphaValue = 0
        detailContainer.passesHitTests = false
        listView.isHidden = false
        listView.alphaValue = 1
        listView.passesHitTests = true
        onHomeSessionSaved?()
    }

    func resetResults() {
        if !showingDetail { mode = .home }
        currentItems = []
        selectedIndex = 0
        listView.clear()
    }

    func showHome(_ snapshot: LauncherHomeSnapshot, preserveSelection: Bool = false) {
        if !showingDetail { mode = .home }
        currentItems = snapshot.flatItems
        if !preserveSelection {
            selectedIndex = 0
        }
        listView.renderHome(snapshot, preserveSelection: preserveSelection)
        syncSelectionFromList()
        LauncherInPanelLayout.stabilizePanel(from: listView)
    }

    func present(
        _ detail: any ModuleDetailView,
        moduleID: ModuleIdentifier,
        presentation: ModuleDetailPresentation = .rightColumn,
        prefillTranslateText: String? = nil,
        stagedForCrossfade: Bool = false
    ) {
        let contentView = detail.detailView
        let reusingHierarchy = contentView.superview === detailContainer

        if reusingHierarchy {
            currentDetailObject = detail
            detailModuleIDStorage = moduleID
            contentView.isHidden = false
            detailTopBar.isHidden = !detail.usesSharedTopBar
        } else {
            if currentDetailObject !== detail {
                currentDetailObject?.deactivate()
                currentDetailObject?.detailView.removeFromSuperview()
            }
            for subview in detailContainer.subviews where subview !== contentView {
                subview.removeFromSuperview()
            }
            currentDetailObject = detail
            detailModuleIDStorage = moduleID

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
        }

        detailTitleLabel.stringValue = detail.moduleTitle
        mode = .detail(moduleID)
        detailContainer.isHidden = false
        detailContainer.alphaValue = stagedForCrossfade ? 0 : 1
        detailContainer.passesHitTests = !stagedForCrossfade
        listView.isHidden = false
        listView.alphaValue = 1
        listView.passesHitTests = true
        LauncherInPanelLayout.stabilizePanel(from: detailContainer)

        if moduleID == .translate, let text = prefillTranslateText ?? pendingTranslateText {
            pendingTranslateText = nil
            if let translate = detail as? TranslateDetailView {
                translate.prefill(text: text, autoTranslate: true)
            }
        }
        if !reusingHierarchy {
            onSessionChanged?()
        }
    }

    func closeDetail(presentation: ModuleDetailPresentation = .rightColumn) {
        guard showingDetail else { return }
        mode = showingResults ? .results : .home
        currentDetailObject?.deactivate()
        if let pooledView = currentDetailObject?.detailView, pooledView.superview === detailContainer {
            pooledView.isHidden = true
        } else {
            currentDetailObject?.detailView.removeFromSuperview()
        }
        currentDetailObject = nil
        detailModuleIDStorage = nil
        detailTopBar.isHidden = false
        detailContainer.passesHitTests = false
        detailContainer.isHidden = true
        detailContainer.alphaValue = 0
        listView.isHidden = false
        listView.alphaValue = 1
        listView.passesHitTests = true
        onHomeSessionSaved?()
        LauncherInPanelLayout.stabilizePanel(from: listView)
    }

    func renderResults(_ items: [ResultItem], layout: ResultListLayout = .flat) {
        let newItems = Array(items.prefix(8))
        let previouslySelectedID: ResultID? = currentItems[safe: selectedIndex]?.id
        currentItems = newItems
        guard !newItems.isEmpty else {
            mode = .results
            currentItems = []
            selectedIndex = 0
            listView.renderResults([], layout: layout)
            LauncherInPanelLayout.stabilizePanel(from: listView)
            return
        }
        mode = .results
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
        if !showingDetail { mode = .home }
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
        if !showingDetail { mode = .results }
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
