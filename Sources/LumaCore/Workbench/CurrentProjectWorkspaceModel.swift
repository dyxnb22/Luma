import Foundation

public struct CurrentProjectWorkspaceCaptureAction: Sendable, Equatable {
    public let title: String
    public let source: WorkbenchCaptureSource
    public let target: WorkbenchCaptureTarget
    public let moduleID: ModuleIdentifier

    public init(
        title: String,
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        moduleID: ModuleIdentifier
    ) {
        self.title = title
        self.source = source
        self.target = target
        self.moduleID = moduleID
    }
}

public struct CurrentProjectWorkspaceActivityLine: Sendable, Equatable {
    public let title: String
    public let subtitle: String

    public init(title: String, subtitle: String) {
        self.title = title
        self.subtitle = subtitle
    }
}

/// Pure view model for the project workspace detail surface.
public struct CurrentProjectWorkspaceModel: Sendable, Equatable {
    public let headerTitle: String
    public let headerLines: [String]
    public let quickCaptureActions: [CurrentProjectWorkspaceCaptureAction]
    public let quickCaptureDisabledHint: String?
    public let recentActivityLines: [CurrentProjectWorkspaceActivityLine]
    public let showsProjectActions: Bool

    public init(
        headerTitle: String,
        headerLines: [String],
        quickCaptureActions: [CurrentProjectWorkspaceCaptureAction],
        quickCaptureDisabledHint: String?,
        recentActivityLines: [CurrentProjectWorkspaceActivityLine],
        showsProjectActions: Bool
    ) {
        self.headerTitle = headerTitle
        self.headerLines = headerLines
        self.quickCaptureActions = quickCaptureActions
        self.quickCaptureDisabledHint = quickCaptureDisabledHint
        self.recentActivityLines = recentActivityLines
        self.showsProjectActions = showsProjectActions
    }
}

public enum CurrentProjectWorkspaceModelBuilder {
    private static let allCaptureCandidates: [CurrentProjectWorkspaceCaptureAction] = [
        CurrentProjectWorkspaceCaptureAction(
            title: "New project snippet",
            source: .projectContext,
            target: .projectSnippetDraft,
            moduleID: .workbenchSnippets
        ),
        CurrentProjectWorkspaceCaptureAction(
            title: "New project quicklink",
            source: .projectContext,
            target: .quicklinkDraft,
            moduleID: .workbenchQuicklinks
        ),
        CurrentProjectWorkspaceCaptureAction(
            title: "New project todo",
            source: .projectContext,
            target: .todoDraft,
            moduleID: .workbenchTodo
        ),
        CurrentProjectWorkspaceCaptureAction(
            title: "New project note",
            source: .projectContext,
            target: .noteDraft,
            moduleID: .workbenchNotes
        )
    ]

    public static func build(
        context: CurrentProjectContext?,
        activitySnapshot: WorkbenchActivitySnapshot,
        enabledModuleIDs: Set<ModuleIdentifier>,
        existingProjectNotePath: String? = nil
    ) -> CurrentProjectWorkspaceModel {
        guard let context else {
            return empty(context: nil)
        }

        var headerLines = ["In \(context.frontAppName): \(context.projectLabel)"]
        if let path = context.matchedProjectPath {
            headerLines.append(shortenPath(path))
            if let notePath = existingProjectNotePath {
                headerLines.append("Notes: \(shortenPath(notePath))")
            }
        } else {
            headerLines.append("Project path not matched in projects.json.")
        }

        let quickCapture = allCaptureCandidates.filter { enabledModuleIDs.contains($0.moduleID) }
        let disabledHint = quickCapture.isEmpty
            ? "Enable Snippets, Quicklinks, Todo, or Notes in Settings for quick capture."
            : nil

        let activityLines = activitySnapshot.currentProjectRecent
            .filter { enabledModuleIDs.contains($0.moduleID) }
            .map { entry in
            let subtitle = entry.preview ?? entry.detail ?? ""
            return CurrentProjectWorkspaceActivityLine(title: entry.title, subtitle: subtitle)
        }

        return CurrentProjectWorkspaceModel(
            headerTitle: "Project workspace",
            headerLines: headerLines,
            quickCaptureActions: context.matchedProjectPath == nil ? [] : quickCapture,
            quickCaptureDisabledHint: context.matchedProjectPath == nil ? nil : disabledHint,
            recentActivityLines: activityLines,
            showsProjectActions: context.matchedProjectPath != nil
        )
    }

    public static func loading(context: CurrentProjectContext?) -> CurrentProjectWorkspaceModel {
        guard let context else {
            return CurrentProjectWorkspaceModel(
                headerTitle: "Project workspace",
                headerLines: ["Loading…"],
                quickCaptureActions: [],
                quickCaptureDisabledHint: nil,
                recentActivityLines: [],
                showsProjectActions: false
            )
        }
        let name = context.projectName ?? context.projectLabel
        return CurrentProjectWorkspaceModel(
            headerTitle: "Project workspace",
            headerLines: ["Loading \(name)…"],
            quickCaptureActions: [],
            quickCaptureDisabledHint: nil,
            recentActivityLines: [],
            showsProjectActions: false
        )
    }

    private static func empty(context: CurrentProjectContext?) -> CurrentProjectWorkspaceModel {
        CurrentProjectWorkspaceModel(
            headerTitle: "Project workspace",
            headerLines: [context == nil ? "No IDE project detected." : "No project context."],
            quickCaptureActions: [],
            quickCaptureDisabledHint: nil,
            recentActivityLines: [],
            showsProjectActions: false
        )
    }

    private static func shortenPath(_ path: String) -> String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if path.hasPrefix(home) { return "~" + path.dropFirst(home.count) }
        return path
    }
}
