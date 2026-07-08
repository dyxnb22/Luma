import AppKit
import Foundation
import LumaCore

/// Gathers a point-in-time launcher state snapshot for QA export.
@MainActor
enum LauncherStateSnapshotCollector {
    struct Input {
        var panelVisible: Bool
        var visibilitySessionVisible: Bool
        var visibilityGeneration: UInt
        var window: NSWindow?
        var searchBar: LumaSearchBar
        var contentCoordinator: LauncherContentCoordinator
        var homeSplitLayout: LauncherHomeSplitLayout
        var detailContainerHidden: Bool
        var detailContainerAlpha: Double
        var hintContext: String
        var detailPresentationGeneration: UInt
        var detailCloseCrossfadeInFlight: Bool
        var reason: String?
    }

    static func collect(_ input: Input) -> LauncherStateSnapshot {
        let window = input.window
        let firstResponder = describeFirstResponder(window?.firstResponder)

        let split = input.homeSplitLayout.splitLayoutStateForSnapshot
        let mode = input.contentCoordinator.mode
        let selectedItem = input.contentCoordinator.currentItems[
            safe: input.contentCoordinator.selectedIndex
        ]

        let homeVisible = !input.contentCoordinator.showingResults
            && !input.contentCoordinator.showingDetail
        let resultsVisible = input.contentCoordinator.showingResults

        return LauncherStateSnapshot(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            reason: input.reason,
            panel: .init(
                panelVisible: input.panelVisible,
                visibilitySessionVisible: input.visibilitySessionVisible,
                visibilityGeneration: input.visibilityGeneration,
                isKeyWindow: window?.isKeyWindow ?? false,
                firstResponderChain: firstResponder
            ),
            search: .init(
                visibleQuery: input.searchBar.stringValue,
                persistedQuery: input.searchBar.persistedQuery,
                isDetailModeActive: input.searchBar.isDetailModeActive,
                isEditable: input.searchBar.isSearchFieldEditable,
                placeholder: input.searchBar.currentPlaceholderTextForSnapshot
            ),
            content: .init(
                modeKind: mode.snapshotKind,
                detailModuleID: mode.detailModuleID?.rawValue,
                showingDetail: input.contentCoordinator.showingDetail,
                showingResults: input.contentCoordinator.showingResults,
                selectedIndex: input.contentCoordinator.selectedIndex,
                selectedItemID: selectedItem.map { "\($0.id.module.rawValue):\($0.id.key)" },
                currentDetailModuleID: input.contentCoordinator.currentDetailModuleID?.rawValue
            ),
            chrome: .init(
                detailContainerHidden: input.detailContainerHidden,
                detailContainerAlpha: input.detailContainerAlpha,
                splitColumnActive: split.columnSplitActive,
                splitRightPane: split.rightPane.snapshotKind,
                homeVisible: homeVisible,
                resultsVisible: resultsVisible,
                hintContext: input.hintContext
            ),
            animation: .init(
                detailPresentationGeneration: input.detailPresentationGeneration,
                crossfadeGeneration: input.homeSplitLayout.crossfadeGenerationForSnapshot,
                detailCloseCrossfadeInFlight: input.detailCloseCrossfadeInFlight
            ),
            lastKeyboardCommand: LauncherStateKeyboardRecorder.lastCommand,
            searchFieldCanBecomeFirstResponder: input.searchBar.isSearchFieldEditable
                || input.searchBar.isActivelyEditing
        )
    }

    private static func describeFirstResponder(_ responder: NSResponder?) -> String {
        guard let responder else { return "nil" }
        if responder is NSWindow { return "window" }
        let typeName = String(describing: type(of: responder))
        if let view = responder as? NSView, let identifier = view.identifier?.rawValue, !identifier.isEmpty {
            return "\(typeName):\(identifier)"
        }
        return typeName
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
