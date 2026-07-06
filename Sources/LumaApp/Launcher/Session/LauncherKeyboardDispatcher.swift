import AppKit
import LumaCore

/// Routes list/search key commands through `LauncherKeyRouter`.
@MainActor
struct LauncherKeyboardDispatcher {
    struct Context {
        var actionPanelVisible: Bool
        var actionPanelActionCount: Int
        var showingDetail: Bool
        var listHoldsKeyboardFocus: Bool
        var showingResults: Bool
        var queryTrimmedIsEmpty: Bool
        var itemCount: Int
        var selectedIndex: Int
        var currentItem: ResultItem?
        var contentMode: LauncherContentMode

        var dismissActionPanel: () -> Void
        var openActionPanel: () -> Void
        var moveActionPanelSelection: (Int) -> Void
        var activateActionPanelIndex: (Int) -> Void
        var moveListSelection: (Int) -> Void
        var jumpToFlatIndex: (Int) -> Void
        var runItem: (ResultItem) -> Void
        var runSecondaryForSelected: () -> Bool
        var showNoAlternateActions: () -> Void
    }

    static func handle(_ command: LumaSearchBar.KeyCommand, context: Context) -> Bool {
        if case .backtab = command {
            if context.actionPanelVisible {
                context.dismissActionPanel()
                return true
            }
            return false
        }
        if case .commandReturn = command {
            guard !context.actionPanelVisible, !context.showingDetail,
                  let item = context.currentItem else { return false }
            guard item.secondaryActions.first != nil else {
                context.showNoAlternateActions()
                return true
            }
            return context.runSecondaryForSelected()
        }
        if context.actionPanelVisible {
            if case .commandNumber(let number) = command {
                context.activateActionPanelIndex(number - 1)
                return true
            }
            if case .up = command {
                context.moveActionPanelSelection(-1)
                return true
            }
            if case .down = command {
                context.moveActionPanelSelection(1)
                return true
            }
            let outcome = LauncherKeyRouter.route(
                command: command.launcherKeyCommand,
                mode: .results,
                itemCount: context.actionPanelActionCount,
                actionPanelVisible: true
            )
            if outcome == .dismissActionPanel {
                context.dismissActionPanel()
                return true
            }
            return true
        }

        let mode: LauncherContentMode
        if context.showingDetail, !context.listHoldsKeyboardFocus {
            mode = context.contentMode
        } else if context.showingResults || !context.queryTrimmedIsEmpty {
            mode = .results
        } else {
            mode = .home
        }

        let outcome = LauncherKeyRouter.route(
            command: command.launcherKeyCommand,
            mode: mode,
            itemCount: context.itemCount,
            actionPanelVisible: context.actionPanelVisible
        )
        switch outcome {
        case .handled: return true
        case .openActionPanel:
            context.openActionPanel()
            return true
        case .moveSelection(let delta):
            context.moveListSelection(delta)
            return true
        case .jumpToFlatIndex(let index):
            context.jumpToFlatIndex(index)
            return true
        case .runItem(let item):
            context.runItem(item)
            return true
        case .toggleOpenAppWindows:
            return true
        case .dismissActionPanel:
            context.dismissActionPanel()
            return true
        case .passthrough:
            return false
        }
    }
}

private extension LumaSearchBar.KeyCommand {
    var launcherKeyCommand: LauncherKeyCommand {
        switch self {
        case .up: .up
        case .down: .down
        case .tab: .tab
        case .backtab: .backtab
        case .actionPanel: .actionPanel
        case .commandReturn: .commandReturn
        case .commandNumber(let n): .commandNumber(n)
        }
    }
}
