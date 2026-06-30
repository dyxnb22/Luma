import Foundation

/// Resolves activity entries into lightweight entity references without module store access.
public enum WorkbenchEntityResolver {
    public static func resolve(_ entry: WorkbenchActivityEntry) -> WorkbenchEntityRef? {
        if let entityRef = entry.entityRef {
            return entityRef
        }
        if let payload = entry.resumablePayload {
            return draftRef(for: entry, payload: payload)
        }
        if let entityKind = entry.entityKind, let entityID = entry.entityID {
            return WorkbenchEntityRef(
                kind: entityKind,
                entityID: entityID,
                moduleID: entry.moduleID,
                title: entry.title,
                subtitle: entry.preview ?? entry.detail
            )
        }
        if entry.resumeRef?.kind == .noteAction {
            return WorkbenchEntityRef(
                kind: .note,
                entityID: entry.id.uuidString,
                moduleID: entry.moduleID,
                title: entry.title,
                subtitle: entry.preview ?? entry.detail
            )
        }
        return nil
    }

    private static func draftRef(
        for entry: WorkbenchActivityEntry,
        payload: WorkbenchActivityResumePayload
    ) -> WorkbenchEntityRef? {
        let kind: WorkbenchEntityKind = switch payload {
        case .snippetDraft: .snippet
        case .quicklinkDraft: .quicklink
        case .todoCapture: .todo
        case .noteReference: .note
        }
        return WorkbenchEntityRef(
            kind: kind,
            entityID: entry.id.uuidString,
            moduleID: entry.moduleID,
            title: entry.title,
            subtitle: entry.preview ?? entry.detail
        )
    }
}
