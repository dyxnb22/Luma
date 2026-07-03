import Foundation
import LumaCore
import LumaModules

enum LauncherActionFeedback {
  static func feedback(for action: Action) -> LauncherFeedback? {
    switch action.kind {
    case .openModuleDetail(let moduleID, let payload):
      guard let payload else { return nil }
      if moduleID == .snippets,
         let snippetsAction = try? ModuleActionCoding.decode(SnippetsAction.self, from: payload),
         case .prepareDraft = snippetsAction {
        return LauncherFeedback(kind: .success, message: LauncherStatusMessages.draftLoadedInSnippets, delayDismiss: false)
      }
      if moduleID == .quicklinks,
         let quicklinksAction = try? ModuleActionCoding.decode(QuicklinksAction.self, from: payload),
         case .prepareDraft = quicklinksAction {
        return LauncherFeedback(kind: .success, message: LauncherStatusMessages.draftLoadedInQuicklinks, delayDismiss: false)
      }
      return nil
    case .custom:
      return nil
    case .copyToPasteboard:
      return LauncherFeedback(kind: .success, message: LauncherStatusMessages.copiedToClipboard, delayDismiss: true)
    default:
      return nil
    }
  }

  static func statusMessage(for action: Action) -> String? {
    feedback(for: action)?.message
  }

  static func shouldDelayDismiss(for action: Action) -> Bool {
    feedback(for: action)?.delayDismiss ?? false
  }
}
