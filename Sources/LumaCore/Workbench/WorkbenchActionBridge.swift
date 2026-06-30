import Foundation

/// Maps semantic workbench actions to launcher `Action` values.
public enum WorkbenchActionBridge {
    public static func launcherAction(
        for workbench: WorkbenchAction,
        actionKey: String
    ) -> Action {
        switch workbench.kind {
        case .open, .continueWork, .create, .prepareDraft:
            return Action(
                id: ActionID(module: workbench.targetModule, key: actionKey),
                title: workbench.title,
                kind: .openModuleDetail(workbench.targetModule, payload: workbench.payload)
            )
        case .capture, .convert:
            if let payload = workbench.payload {
                return Action(
                    id: ActionID(module: workbench.targetModule, key: actionKey),
                    title: workbench.title,
                    kind: .custom(payload: payload, handler: workbench.targetModule)
                )
            }
            return Action(
                id: ActionID(module: workbench.targetModule, key: actionKey),
                title: workbench.title,
                kind: .openModuleDetail(workbench.targetModule, payload: nil)
            )
        case .linkToProject, .pin, .archive:
            if let payload = workbench.payload {
                return Action(
                    id: ActionID(module: workbench.targetModule, key: actionKey),
                    title: workbench.title,
                    kind: .custom(payload: payload, handler: workbench.targetModule)
                )
            }
            return Action(
                id: ActionID(module: workbench.targetModule, key: actionKey),
                title: workbench.title,
                kind: .noop
            )
        }
    }
}
