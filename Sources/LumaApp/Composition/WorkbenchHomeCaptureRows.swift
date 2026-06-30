import Foundation
import LumaCore
import LumaModules

/// Builds Home row actions that run through the workbench capture pipeline.
enum WorkbenchHomeCaptureRows {
    static func captureAction(
        key: String,
        title: String,
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        moduleID: ModuleIdentifier
    ) -> Action {
        let payload = (try? ModuleActionCoding.encode(
            WorkbenchCaptureAction.prepareDraft(source: source, target: target)
        )) ?? Data()
        return Action(
            id: ActionID(module: moduleID, key: key),
            title: title,
            kind: .custom(payload: payload, handler: .workbench)
        )
    }

    static func resumeAction(entry: WorkbenchActivityEntry) -> Action {
        let payload = (try? ModuleActionCoding.encode(
            WorkbenchCaptureAction.resumeActivity(entryID: entry.id)
        )) ?? Data()
        return Action(
            id: ActionID(module: entry.moduleID, key: "contextual.project-activity.\(entry.id.uuidString)"),
            title: entry.title,
            kind: .custom(payload: payload, handler: .workbench)
        )
    }
}
