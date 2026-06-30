import Foundation

public enum WorkbenchCaptureSource: String, Sendable, Codable, CaseIterable {
    case selection
    case clipboardText
    case clipboardURL
    case projectContext
}

public enum WorkbenchCaptureTarget: String, Sendable, Codable, CaseIterable {
    case noteDraft
    case todoDraft
    case snippetDraft
    case quicklinkDraft
    case projectSnippetDraft

    public var displayName: String {
        switch self {
        case .noteDraft: "note draft"
        case .todoDraft: "todo draft"
        case .snippetDraft: "snippet draft"
        case .quicklinkDraft: "quicklink draft"
        case .projectSnippetDraft: "project snippet draft"
        }
    }

    public var moduleID: ModuleIdentifier {
        switch self {
        case .noteDraft: .workbenchNotes
        case .todoDraft: .workbenchTodo
        case .snippetDraft: .workbenchSnippets
        case .quicklinkDraft: .workbenchQuicklinks
        case .projectSnippetDraft: .workbenchSnippets
        }
    }
}

public struct WorkbenchCaptureResult: Sendable {
    public let target: WorkbenchCaptureTarget
    public let moduleID: ModuleIdentifier
    public let preview: String
    public let actionPayload: Data?
    public let openDetailPayload: Data?
    public let resumeDraftJSON: Data?

    public init(
        target: WorkbenchCaptureTarget,
        moduleID: ModuleIdentifier,
        preview: String,
        actionPayload: Data? = nil,
        openDetailPayload: Data? = nil,
        resumeDraftJSON: Data? = nil
    ) {
        self.target = target
        self.moduleID = moduleID
        self.preview = preview
        self.actionPayload = actionPayload
        self.openDetailPayload = openDetailPayload
        self.resumeDraftJSON = resumeDraftJSON
    }
}

public protocol WorkbenchCaptureService: Sendable {
    func capture(
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        context: WorkbenchContext
    ) async -> WorkbenchCaptureResult?
}
