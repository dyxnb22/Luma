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

public enum LauncherContentMode: Equatable, Sendable {
    case home
    case results
    case detail(ModuleIdentifier? = nil)

    public var showingDetail: Bool {
        if case .detail = self { return true }
        return false
    }

    public var showingResults: Bool {
        if case .results = self { return true }
        return false
    }

    public var detailModuleID: ModuleIdentifier? {
        if case .detail(let id) = self { return id }
        return nil
    }
}

public enum LauncherKeyRouter {
    public enum Outcome: Equatable {
        case handled
        case openActionPanel
        case dismissActionPanel
        case moveSelection(delta: Int)
        case jumpToFlatIndex(Int)
        case runItem(ResultItem)
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
            guard !mode.showingDetail else { return .handled }
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: 1)
        case .up:
            guard !mode.showingDetail else { return .handled }
            guard itemCount > 0 else { return .handled }
            return .moveSelection(delta: -1)
        case .backtab:
            return .passthrough
        case .tab, .actionPanel:
            guard !mode.showingDetail, itemCount > 0 else { return .handled }
            return .openActionPanel
        case .commandReturn:
            return .handled
        case .commandNumber(let number):
            guard !mode.showingDetail else { return .handled }
            let index = number - 1
            guard itemCount > index, index >= 0 else { return .handled }
            return .jumpToFlatIndex(index)
        }
    }

    public static func resolveRun(item: ResultItem) -> Outcome {
        if item.id.key.hasPrefix(OpenAppsResultBuilder.toggleWindowsKeyPrefix) {
            let bundleID = String(item.id.key.dropFirst(OpenAppsResultBuilder.toggleWindowsKeyPrefix.count))
            return .toggleOpenAppWindows(bundleID)
        }
        return .runItem(item)
    }
}
