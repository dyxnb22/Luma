import Foundation

public extension Action {
    /// Short label for the command hint bar describing what Return does on this action.
    var returnHint: String {
        switch kind {
        case .launchApp:
            return "Launch app"
        case .focusWindow:
            return "Focus window"
        case .copyToPasteboard:
            return "Copy"
        case .openURL:
            return "Open link"
        case .revealInFinder:
            return "Reveal in Finder"
        case .insertText:
            return "Insert text"
        case .applyWindowLayout:
            return "Move window"
        case .translateText:
            return "Open Translate"
        case .openModuleDetail:
            return title.isEmpty ? "Open detail" : title
        case .replaceQuery:
            return "Replace query"
        case .custom(_, let handler):
            return "Run \(handler.rawValue.replacingOccurrences(of: "luma.", with: ""))"
        case .noop:
            return "No action"
        }
    }
}

public extension ResultItem {
    var returnHint: String {
        switch rowKind {
        case .informational:
            return "Information only"
        case .actionable:
            return primaryAction.returnHint
        }
    }
}
