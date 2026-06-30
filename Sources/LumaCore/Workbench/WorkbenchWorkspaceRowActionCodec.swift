import Foundation

/// Encodes workspace row actions into launcher `Action` payloads and command outcomes.
public enum WorkbenchWorkspaceRowActionCodec {
    public static func launcherAction(
        for rowAction: CurrentProjectWorkspaceRowAction,
        entry: WorkbenchActivityEntry,
        key: String
    ) -> Action {
        let actionID = ActionID(module: entry.moduleID, key: key)
        let title = entry.title
        switch rowAction {
        case .replaceQuery(let text):
            return Action(id: actionID, title: title, kind: .replaceQuery(text))
        case .openModule(let moduleID):
            return Action(id: actionID, title: title, kind: .openModuleDetail(moduleID, payload: nil))
        case .resumeActivity(let entryID):
            let payload = (try? ModuleActionCoding.encode(
                WorkbenchCaptureAction.resumeActivity(entryID: entryID)
            )) ?? Data()
            if payload.isEmpty {
                return fallbackActivityEntryAction(entry: entry, key: key)
            }
            return Action(
                id: actionID,
                title: title,
                kind: .custom(payload: payload, handler: .workbench)
            )
        case .openLinked(let linkID):
            let payload = (try? ModuleActionCoding.encode(
                WorkbenchEntityAction.openLinked(linkID: linkID)
            )) ?? Data()
            return Action(
                id: actionID,
                title: title,
                kind: .custom(payload: payload, handler: .workbench)
            )
        case .openNotePath:
            return Action(id: actionID, title: title, kind: .openModuleDetail(.workbenchNotes, payload: nil))
        case .status(let message):
            let payload = (try? ModuleActionCoding.encode(
                WorkbenchEntityAction.showStatus(message)
            )) ?? Data()
            return Action(
                id: actionID,
                title: title,
                kind: .custom(payload: payload, handler: .workbench)
            )
        }
    }

    public static func commandOutcome(
        for rowAction: CurrentProjectWorkspaceRowAction,
        entry: WorkbenchActivityEntry
    ) -> WorkbenchCommandOutcome? {
        switch rowAction {
        case .replaceQuery(let text):
            return .replaceQuery(text)
        case .openModule(let moduleID):
            return .openDetail(moduleID, payload: nil)
        case .resumeActivity(let entryID):
            return .resumeActivity(entryID)
        case .openLinked(let linkID):
            return .openLinked(linkID)
        case .openNotePath:
            return .openDetail(.workbenchNotes, payload: nil)
        case .status(let message):
            return .status(message)
        }
    }

    private static func fallbackActivityEntryAction(
        entry: WorkbenchActivityEntry,
        key: String
    ) -> Action {
        let payload = (try? ModuleActionCoding.encode(
            WorkbenchEntityAction.openActivityEntry(entryID: entry.id)
        )) ?? Data()
        return Action(
            id: ActionID(module: entry.moduleID, key: key),
            title: entry.title,
            kind: .custom(payload: payload, handler: .workbench)
        )
    }
}
