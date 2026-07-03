import Foundation

public enum LauncherKeyCommand: Sendable, Equatable {
    case up
    case down
    case tab
    case backtab
    case actionPanel
    case commandReturn
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
        case dismissActionPanel
        case moveSelection(delta: Int)
        case jumpToFlatIndex(Int)
        case runItem(ResultItem)
        case expandOpenApps
        case toggleOpenAppWindows(String)
        case passthrough
    }

    public static func route(
        command: LauncherKeyCommand,
        mode: LauncherContentMode,
        itemCount: Int,
        actionPanelVisible: Bool
    ) -> Outcome {
        if actionPanelVisible {
            switch command {
            case .tab, .backtab:
                return .dismissActionPanel
            default:
                break
            }
        }

        switch command {
        case .down:
            guard mode != .detail else { return .handled }
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: 1)
        case .up:
            guard mode != .detail else { return .handled }
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: -1)
        case .backtab:
            return .passthrough
        case .tab, .actionPanel:
            guard mode != .detail, itemCount > 0 else { return .handled }
            return .openActionPanel
        case .commandReturn:
            return .handled
        case .commandNumber(let number):
            guard mode != .detail else { return .handled }
            let index = number - 1
            guard itemCount > index, index >= 0 else { return .handled }
            return .jumpToFlatIndex(index)
        }
    }

    public static func resolveRun(item: ResultItem) -> Outcome {
        if item.id.key == "openApps.more" || item.primaryAction.id.key == "openApps.expand" {
            return .expandOpenApps
        }
        if item.id.key.hasPrefix(OpenAppsResultBuilder.toggleWindowsKeyPrefix) {
            let bundleID = String(item.id.key.dropFirst(OpenAppsResultBuilder.toggleWindowsKeyPrefix.count))
            return .toggleOpenAppWindows(bundleID)
        }
        return .runItem(item)
    }
}
