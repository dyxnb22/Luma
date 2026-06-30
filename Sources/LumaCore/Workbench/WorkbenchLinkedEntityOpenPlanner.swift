import Foundation

/// Maps linked entities and activity entries to workspace row actions.
public enum WorkbenchLinkedEntityOpenPlanner {
    public static func rowAction(
        for link: WorkbenchProjectLink,
        entry: WorkbenchActivityEntry?
    ) -> CurrentProjectWorkspaceRowAction {
        if let entry {
            return rowAction(for: entry)
        }
        return rowAction(for: link.entityRef)
    }

    public static func rowAction(for entry: WorkbenchActivityEntry) -> CurrentProjectWorkspaceRowAction {
        if let payload = entry.resumablePayload {
            switch payload {
            case .snippetDraft, .quicklinkDraft:
                return .resumeActivity(entryID: entry.id)
            case .todoCapture(let text):
                return .replaceQuery(TodoModuleResumeQuery.resumeQuery(forCapture: text))
            case .noteReference:
                return .openModule(moduleID: .workbenchNotes)
            }
        }
        switch resolvedEntityKind(for: entry) {
        case .quicklink, .snippet:
            return .openModule(moduleID: entry.moduleID)
        case .todo:
            if let text = entry.preview, !text.isEmpty {
                return .replaceQuery(TodoModuleResumeQuery.resumeQuery(forCapture: text))
            }
        case .note:
            return .openModule(moduleID: entry.moduleID)
        default:
            break
        }
        return .status("Recorded activity")
    }

    private static func resolvedEntityKind(for entry: WorkbenchActivityEntry) -> WorkbenchEntityKind? {
        entry.entityKind ?? entry.entityRef?.kind
    }

    public static func rowAction(for ref: WorkbenchEntityRef) -> CurrentProjectWorkspaceRowAction {
        switch ref.kind {
        case .quicklink, .snippet, .note, .project:
            return .openModule(moduleID: ref.moduleID)
        case .todo:
            return .status("Open Todo to view linked item")
        case .clipboardItem, .urlReference, .fileReference, .secretReference:
            return .status("Linked item not yet openable")
        }
    }
}
