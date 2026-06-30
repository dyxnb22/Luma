import Foundation

public struct WorkbenchActivityRowPresentation: Sendable, Equatable {
    public let rowAction: CurrentProjectWorkspaceRowAction
    public let subtitle: String
    public let isInteractive: Bool
    public let iconName: String

    public init(
        rowAction: CurrentProjectWorkspaceRowAction,
        subtitle: String,
        isInteractive: Bool,
        iconName: String
    ) {
        self.rowAction = rowAction
        self.subtitle = subtitle
        self.isInteractive = isInteractive
        self.iconName = iconName
    }
}

/// Builds launcher actions for workbench activity rows using the shared open planner.
public enum WorkbenchActivityRowActions {
    public static func presentation(for entry: WorkbenchActivityEntry) -> WorkbenchActivityRowPresentation {
        let rowAction = WorkbenchLinkedEntityOpenPlanner.rowAction(for: entry)
        let preview = entry.preview ?? entry.detail ?? ""
        let isInteractive: Bool
        let subtitle: String
        let iconName: String

        switch rowAction {
        case .status(let message):
            isInteractive = false
            subtitle = preview.isEmpty ? message : preview
            iconName = "clock"
        case .resumeActivity:
            isInteractive = true
            subtitle = preview.isEmpty ? "Press Return to resume" : preview
            iconName = "clock.arrow.circlepath"
        case .replaceQuery:
            isInteractive = true
            subtitle = preview.isEmpty ? "Press Return to open" : preview
            iconName = "arrow.right.circle"
        case .openModule(let moduleID):
            isInteractive = true
            if preview.isEmpty, moduleID == .workbenchNotes {
                subtitle = "Open Notes"
            } else {
                subtitle = preview.isEmpty ? "Press Return to open" : preview
            }
            iconName = "arrow.right.circle"
        case .openNotePath:
            isInteractive = true
            subtitle = preview.isEmpty ? "Open Notes" : preview
            iconName = "arrow.right.circle"
        case .openLinked:
            isInteractive = true
            subtitle = preview.isEmpty ? "Press Return to open" : preview
            iconName = "arrow.right.circle"
        }

        if entry.isResumableDraft {
            return WorkbenchActivityRowPresentation(
                rowAction: rowAction,
                subtitle: preview.isEmpty ? "Press Return to resume" : preview,
                isInteractive: isInteractive,
                iconName: "clock.arrow.circlepath"
            )
        }

        return WorkbenchActivityRowPresentation(
            rowAction: rowAction,
            subtitle: subtitle,
            isInteractive: isInteractive,
            iconName: iconName
        )
    }

    public static func isInteractive(_ entry: WorkbenchActivityEntry) -> Bool {
        presentation(for: entry).isInteractive
    }

    public static func primaryAction(for entry: WorkbenchActivityEntry, key: String) -> Action {
        let row = presentation(for: entry)
        return WorkbenchWorkspaceRowActionCodec.launcherAction(
            for: row.rowAction,
            entry: entry,
            key: key
        )
    }
}
