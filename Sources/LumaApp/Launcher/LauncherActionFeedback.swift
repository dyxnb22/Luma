import Foundation
import LumaCore
import LumaModules

enum LauncherActionFeedback {
    static func statusMessage(for action: Action) -> String? {
        switch action.kind {
        case .openModuleDetail(let moduleID, let payload):
            guard let payload else { return nil }
            if moduleID == .snippets,
               let snippetsAction = try? ModuleActionCoding.decode(SnippetsAction.self, from: payload),
               case .prepareDraft = snippetsAction {
                return LauncherStatusMessages.draftLoadedInSnippets
            }
            if moduleID == .quicklinks,
               let quicklinksAction = try? ModuleActionCoding.decode(QuicklinksAction.self, from: payload),
               case .prepareDraft = quicklinksAction {
                return LauncherStatusMessages.draftLoadedInQuicklinks
            }
            return nil
        case .custom:
            return nil
        default:
            return nil
        }
    }

    static func shouldDelayDismiss(for action: Action) -> Bool {
        statusMessage(for: action) != nil
    }
}
