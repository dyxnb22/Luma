import Foundation

public enum CurrentProjectWorkspaceRowAction: Sendable, Equatable, Codable {
    case resumeActivity(entryID: UUID)
    case openLinked(linkID: UUID)
    case openModule(moduleID: ModuleIdentifier)
    case replaceQuery(String)
    case openNotePath(String)
    case status(String)
}

public struct CurrentProjectWorkspaceLinkedRow: Sendable, Equatable {
    public let title: String
    public let subtitle: String
    public let action: CurrentProjectWorkspaceRowAction
    public let moduleID: ModuleIdentifier

    public init(
        title: String,
        subtitle: String,
        action: CurrentProjectWorkspaceRowAction,
        moduleID: ModuleIdentifier
    ) {
        self.title = title
        self.subtitle = subtitle
        self.action = action
        self.moduleID = moduleID
    }
}

public struct CurrentProjectWorkspaceActionRow: Sendable, Equatable {
    public let title: String
    public let subtitle: String
    public let action: CurrentProjectWorkspaceRowAction
    public let entryID: UUID?
    public let isInteractive: Bool

    public init(
        title: String,
        subtitle: String,
        action: CurrentProjectWorkspaceRowAction,
        entryID: UUID? = nil,
        isInteractive: Bool = false
    ) {
        self.title = title
        self.subtitle = subtitle
        self.action = action
        self.entryID = entryID
        self.isInteractive = isInteractive
    }
}

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
    public let linkedItemRows: [CurrentProjectWorkspaceLinkedRow]
    public let recentActivityRows: [CurrentProjectWorkspaceActionRow]
    public let recentActivityLines: [CurrentProjectWorkspaceActivityLine]
    public let showsProjectActions: Bool

    public init(
        headerTitle: String,
        headerLines: [String],
        quickCaptureActions: [CurrentProjectWorkspaceCaptureAction],
        quickCaptureDisabledHint: String?,
        linkedItemRows: [CurrentProjectWorkspaceLinkedRow],
        recentActivityRows: [CurrentProjectWorkspaceActionRow],
        recentActivityLines: [CurrentProjectWorkspaceActivityLine],
        showsProjectActions: Bool
    ) {
        self.headerTitle = headerTitle
        self.headerLines = headerLines
        self.quickCaptureActions = quickCaptureActions
        self.quickCaptureDisabledHint = quickCaptureDisabledHint
        self.linkedItemRows = linkedItemRows
        self.recentActivityRows = recentActivityRows
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
        linkSnapshot: WorkbenchLinkSnapshot = WorkbenchLinkSnapshot(),
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
            ? WorkbenchEmptyStateCopy.captureModulesDisabled
            : nil

        let linkedRows = linkSnapshot.enabledLinks(enabledModuleIDs: enabledModuleIDs, limit: 5)
            .compactMap { linkRow(for: $0) }

        let recentRows = activitySnapshot.currentProjectRecent
            .filter { enabledModuleIDs.contains($0.moduleID) }
            .map { actionRow(for: $0) }

        let activityLines = recentRows.map {
            CurrentProjectWorkspaceActivityLine(title: $0.title, subtitle: $0.subtitle)
        }

        return CurrentProjectWorkspaceModel(
            headerTitle: "Project workspace",
            headerLines: headerLines,
            quickCaptureActions: context.matchedProjectPath == nil ? [] : quickCapture,
            quickCaptureDisabledHint: context.matchedProjectPath == nil ? nil : disabledHint,
            linkedItemRows: linkedRows,
            recentActivityRows: recentRows,
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
                linkedItemRows: [],
                recentActivityRows: [],
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
            linkedItemRows: [],
            recentActivityRows: [],
            recentActivityLines: [],
            showsProjectActions: false
        )
    }

    static func actionRow(for entry: WorkbenchActivityEntry) -> CurrentProjectWorkspaceActionRow {
        let row = WorkbenchActivityRowActions.presentation(for: entry)
        return CurrentProjectWorkspaceActionRow(
            title: entry.title,
            subtitle: row.subtitle,
            action: row.rowAction,
            entryID: entry.id,
            isInteractive: row.isInteractive
        )
    }

    private static func linkRow(for link: WorkbenchProjectLink) -> CurrentProjectWorkspaceLinkedRow? {
        let ref = link.entityRef
        let subtitle = ref.subtitle ?? ""
        return CurrentProjectWorkspaceLinkedRow(
            title: ref.title,
            subtitle: subtitle,
            action: .openLinked(linkID: link.id),
            moduleID: ref.moduleID
        )
    }

    private static func empty(context: CurrentProjectContext?) -> CurrentProjectWorkspaceModel {
        CurrentProjectWorkspaceModel(
            headerTitle: "Project workspace",
            headerLines: [context == nil ? WorkbenchEmptyStateCopy.noIDEProject : WorkbenchEmptyStateCopy.noProjectContext],
            quickCaptureActions: [],
            quickCaptureDisabledHint: nil,
            linkedItemRows: [],
            recentActivityRows: [],
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

/// Avoids LumaCore depending on LumaModules for todo resume query formatting.
public enum TodoModuleResumeQuery {
    public static func resumeQuery(forCapture text: String) -> String {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "t " }
        if trimmed.lowercased().hasPrefix("t ") || trimmed.lowercased().hasPrefix("todo ") {
            return trimmed
        }
        return "t \(trimmed)"
    }
}
