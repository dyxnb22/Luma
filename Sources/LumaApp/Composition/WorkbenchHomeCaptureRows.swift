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
        resumeAction(entryID: entry.id, moduleID: entry.moduleID, title: entry.title, key: "contextual.project-activity.\(entry.id.uuidString)")
    }

    static func resumeAction(entryID: UUID, moduleID: ModuleIdentifier, title: String? = nil, key: String? = nil) -> Action {
        let payload = (try? ModuleActionCoding.encode(
            WorkbenchCaptureAction.resumeActivity(entryID: entryID)
        )) ?? Data()
        return Action(
            id: ActionID(module: moduleID, key: key ?? "contextual.project-activity.\(entryID.uuidString)"),
            title: title ?? "Resume activity",
            kind: .custom(payload: payload, handler: .workbench)
        )
    }

    static func openLinkedAction(link: WorkbenchProjectLink) -> Action {
        let ref = link.entityRef
        let payload = (try? ModuleActionCoding.encode(
            WorkbenchEntityAction.openLinked(linkID: link.id)
        )) ?? Data()
        return Action(
            id: ActionID(module: ref.moduleID, key: "contextual.project-link.\(link.id.uuidString)"),
            title: ref.title,
            kind: .custom(payload: payload, handler: .workbench)
        )
    }
}
