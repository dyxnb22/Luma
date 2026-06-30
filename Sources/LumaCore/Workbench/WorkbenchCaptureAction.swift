import Foundation

/// Encodes a workbench capture intent for Home rows and action dispatch.
public enum WorkbenchCaptureAction: Codable, Sendable, Equatable {
    case prepareDraft(source: WorkbenchCaptureSource, target: WorkbenchCaptureTarget)
    case resumeActivity(entryID: UUID)
}

/// Encodes a workbench command to run on Return.
public enum WorkbenchCommandAction: Codable, Sendable, Equatable {
    case execute(WorkbenchCommandID)
}
