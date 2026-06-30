import Foundation

/// Builds launcher actions for workbench activity rows using the shared open planner.
public enum WorkbenchActivityRowActions {
    public static func isInteractive(_ entry: WorkbenchActivityEntry) -> Bool {
        if case .status = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry) {
            return false
        }
        return true
    }

    public static func primaryAction(for entry: WorkbenchActivityEntry, key: String) -> Action {
        guard isInteractive(entry) else {
            return Action(
                id: ActionID(module: entry.moduleID, key: key),
                title: entry.title,
                kind: .noop
            )
        }
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
