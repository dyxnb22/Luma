import Foundation

public enum LauncherKeyCommand: Sendable, Equatable {
    case up
    case down
    case tab
    case actionPanel
    case commandNumber(Int)
}

public enum LauncherContentMode: Sendable {
    case home
    case results
    case detail
}

public enum LauncherKeyRouter {
    public enum Outcome: Equatable {
        case handled
        case openActionPanel
        case moveSelection(delta: Int)
        case jumpToFlatIndex(Int)
        case runItem(ResultItem)
        case expandOpenApps
        case openTodoDetail
        case openClipboardDetail
        case openRecordsDetail
        case passthrough
    }

    public static func route(
        command: LauncherKeyCommand,
        mode: LauncherContentMode,
        itemCount: Int,
        actionPanelVisible: Bool
    ) -> Outcome {
        if actionPanelVisible, case .tab = command {
            return .handled
        }

        switch command {
        case .down:
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: 1)
        case .up:
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: -1)
        case .tab, .actionPanel:
            guard mode != .detail, itemCount > 0 else { return .handled }
            return .openActionPanel
        case .commandNumber(let number):
            let index = number - 1
            guard itemCount > index, index >= 0 else { return .handled }
            return .jumpToFlatIndex(index)
        }
    }

    public static func resolveRun(item: ResultItem) -> Outcome {
        if item.id.key == "openApps.more" || item.primaryAction.id.key == "openApps.expand" {
            return .expandOpenApps
        }
        if item.id.module.rawValue == "luma.todo", item.id.key.hasPrefix("contextual") {
            return .openTodoDetail
        }
        if item.id.module.rawValue == "luma.clipboard", item.id.key.hasPrefix("contextual") {
            return .openClipboardDetail
        }
        if item.id.module.rawValue == "luma.media", item.id.key.hasPrefix("contextual") {
            return .openRecordsDetail
        }
        return .runItem(item)
    }
}
