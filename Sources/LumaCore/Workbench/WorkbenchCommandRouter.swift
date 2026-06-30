import Foundation

public enum WorkbenchCommandID: String, Sendable, Codable, CaseIterable {
    case captureClipboardNote
    case captureClipboardTodo
    case captureClipboardQuicklink
    case captureSelectionNote
    case captureSelectionTodo
    case continueProject
    case attachProject
    case projectWork
    case projectRecent
    case projectNote
    case projectTodo
    case attachClipboard
    case attachSelection
    case projectLinks
    case projectResume
    case projectCapture
    case projectOpen
    case projectStatus
}

public struct WorkbenchCommandDefinition: Sendable, Hashable {
    public let id: WorkbenchCommandID
    public let triggers: [String]
    public let requiredModule: ModuleIdentifier
    public let captureSource: WorkbenchCaptureSource?
    public let captureTarget: WorkbenchCaptureTarget?
    public let title: String

    public init(
        id: WorkbenchCommandID,
        triggers: [String],
        requiredModule: ModuleIdentifier,
        captureSource: WorkbenchCaptureSource? = nil,
        captureTarget: WorkbenchCaptureTarget? = nil,
        title: String
    ) {
        self.id = id
        self.triggers = triggers
        self.requiredModule = requiredModule
        self.captureSource = captureSource
        self.captureTarget = captureTarget
        self.title = title
    }
}

public enum WorkbenchCommandRoute: Sendable, Equatable {
    case none
    case capture(WorkbenchCommandDefinition)
    case continueProject
    case attachProject
    case projectWork
    case projectRecent
    case attachClipboard
    case attachSelection
    case projectLinks
    case projectResume
    case projectCapture
    case projectOpen
    case projectStatus
}

/// Routes workbench-specific commands before module/global search dispatch.
public struct WorkbenchCommandRouter: Sendable {
    public static let defaultDefinitions: [WorkbenchCommandDefinition] = [
        WorkbenchCommandDefinition(
            id: .captureClipboardNote,
            triggers: ["cap clip note", "capture clip note"],
            requiredModule: .workbenchNotes,
            captureSource: .clipboardText,
            captureTarget: .noteDraft,
            title: "Capture clipboard to note"
        ),
        WorkbenchCommandDefinition(
            id: .captureClipboardTodo,
            triggers: ["cap clip todo", "capture clip todo"],
            requiredModule: .workbenchTodo,
            captureSource: .clipboardText,
            captureTarget: .todoDraft,
            title: "Capture clipboard to todo"
        ),
        WorkbenchCommandDefinition(
            id: .captureClipboardQuicklink,
            triggers: ["cap clip ql", "save url", "capture clip ql"],
            requiredModule: .workbenchQuicklinks,
            captureSource: .clipboardURL,
            captureTarget: .quicklinkDraft,
            title: "Save clipboard URL as quicklink"
        ),
        WorkbenchCommandDefinition(
            id: .captureSelectionNote,
            triggers: ["cap sel note", "capture sel note"],
            requiredModule: .workbenchNotes,
            captureSource: .selection,
            captureTarget: .noteDraft,
            title: "Capture selection to note"
        ),
        WorkbenchCommandDefinition(
            id: .captureSelectionTodo,
            triggers: ["cap sel todo", "capture sel todo"],
            requiredModule: .workbenchTodo,
            captureSource: .selection,
            captureTarget: .todoDraft,
            title: "Capture selection to todo"
        ),
        WorkbenchCommandDefinition(
            id: .projectNote,
            triggers: ["proj note"],
            requiredModule: .workbenchNotes,
            captureSource: .projectContext,
            captureTarget: .noteDraft,
            title: "Capture project note"
        ),
        WorkbenchCommandDefinition(
            id: .projectTodo,
            triggers: ["proj todo"],
            requiredModule: .workbenchTodo,
            captureSource: .projectContext,
            captureTarget: .todoDraft,
            title: "Capture project todo"
        )
    ]

    private let definitions: [WorkbenchCommandDefinition]

    public init(definitions: [WorkbenchCommandDefinition] = WorkbenchCommandRouter.defaultDefinitions) {
        self.definitions = definitions
    }

    public func route(raw: String) -> WorkbenchCommandRoute {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()

        if lower == "continue project" || lower == "proj continue" {
            return .continueProject
        }
        if lower == "proj work" {
            return .projectWork
        }
        if lower == "proj open" {
            return .projectOpen
        }
        if lower == "proj recent" {
            return .projectRecent
        }
        if lower == "proj links" {
            return .projectLinks
        }
        if lower == "proj resume" {
            return .projectResume
        }
        if lower == "proj capture" {
            return .projectCapture
        }
        if lower == "proj status" {
            return .projectStatus
        }
        if lower == "attach project" {
            return .attachProject
        }
        if lower == "attach clip" {
            return .attachClipboard
        }
        if lower == "attach sel" {
            return .attachSelection
        }

        for definition in definitions {
            for trigger in definition.triggers {
                if lower == trigger {
                    return .capture(definition)
                }
            }
        }
        return .none
    }

    public func definition(for id: WorkbenchCommandID) -> WorkbenchCommandDefinition? {
        definitions.first { $0.id == id }
    }
}
