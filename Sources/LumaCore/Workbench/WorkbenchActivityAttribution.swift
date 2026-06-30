import Foundation

public enum WorkbenchActivitySourceKind: String, Sendable, Codable, CaseIterable {
    case clipboard
    case selection
    case project
    case command
    case home
}

public struct WorkbenchProjectAssociation: Sendable, Hashable, Codable {
    public let projectPath: String
    public let projectLabel: String
    public let projectName: String?

    public init(projectPath: String, projectLabel: String, projectName: String? = nil) {
        self.projectPath = projectPath
        self.projectLabel = projectLabel
        self.projectName = projectName
    }

    public init(context: CurrentProjectContext) {
        projectLabel = context.projectLabel
        projectName = context.projectName
        // Prefer real path; legacy unmatched captures stored label in projectPath for query compatibility.
        projectPath = context.matchedProjectPath ?? context.projectLabel
    }
}

public enum WorkbenchResumeRefKind: String, Sendable, Codable, CaseIterable {
    case snippetDraft
    case quicklinkDraft
    case todoCapture
    case noteAction
}

public struct WorkbenchResumeRef: Sendable, Hashable, Codable {
    public let kind: WorkbenchResumeRefKind
    public let entryID: UUID

    public init(kind: WorkbenchResumeRefKind, entryID: UUID) {
        self.kind = kind
        self.entryID = entryID
    }
}

public enum WorkbenchCaptureFollowUp: String, Sendable, Codable {
    case openDetail
    case replaceQuery
    case runNotesAction
    case none
}

public struct WorkbenchCaptureAttribution: Sendable {
    public let sourceKind: WorkbenchActivitySourceKind
    public let followUp: WorkbenchCaptureFollowUp

    public init(sourceKind: WorkbenchActivitySourceKind, followUp: WorkbenchCaptureFollowUp = .none) {
        self.sourceKind = sourceKind
        self.followUp = followUp
    }
}
