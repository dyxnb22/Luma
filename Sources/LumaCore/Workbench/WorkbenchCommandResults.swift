import Foundation

public enum WorkbenchCommandResults {
    public static func previewRows(
        route: WorkbenchCommandRoute,
        querySequence: UInt64,
        context: WorkbenchContext
    ) -> [ResultItem] {
        switch route {
        case .none:
            return []
        case .continueProject, .projectWork:
            return projectWorkspaceRows(
                sequence: querySequence,
                context: context,
                commandID: route == .projectWork ? .projectWork : .continueProject,
                titlePrefix: route == .projectWork ? "Open project workspace" : "Continue project"
            )
        case .projectRecent:
            guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
                return [disabledRow(sequence: querySequence, title: "Project recent activity")]
            }
            let recent = context.activitySnapshot.enabledCurrentProjectRecent(
                enabledModuleIDs: context.enabledModuleIDs,
                limit: 3
            )
            if recent.isEmpty {
                return projectWorkspaceRows(
                    sequence: querySequence,
                    context: context,
                    commandID: .projectRecent,
                    titlePrefix: "Open project workspace"
                )
            }
            return recent.enumerated().map { index, entry in
                activityRow(sequence: querySequence, entry: entry, index: index, commandID: .projectRecent)
            }
        case .attachProject, .attachClipboard, .attachSelection:
            guard context.isEnabled(.workbenchSnippets), context.currentProject != nil else {
                return [disabledRow(sequence: querySequence, title: attachTitle(for: route))]
            }
            let commandID: WorkbenchCommandID = switch route {
            case .attachClipboard: .attachClipboard
            case .attachSelection: .attachSelection
            default: .attachProject
            }
            return [commandRow(
                sequence: querySequence,
                title: attachTitle(for: route),
                subtitle: context.currentProject?.projectLabel ?? "",
                commandID: commandID
            )]
        case .capture(let definition):
            guard context.isEnabled(definition.requiredModule) else {
                return [disabledRow(sequence: querySequence, title: definition.title)]
            }
            return [commandRow(
                sequence: querySequence,
                title: definition.title,
                subtitle: previewSubtitle(for: definition, context: context),
                commandID: definition.id
            )]
        }
    }

    private static func projectWorkspaceRows(
        sequence: UInt64,
        context: WorkbenchContext,
        commandID: WorkbenchCommandID,
        titlePrefix: String
    ) -> [ResultItem] {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return [disabledRow(sequence: sequence, title: titlePrefix)]
        }
        let project = context.currentProject!
        return [commandRow(
            sequence: sequence,
            title: "\(titlePrefix): \(project.projectLabel)",
            subtitle: project.frontAppName,
            commandID: commandID
        )]
    }

    private static func attachTitle(for route: WorkbenchCommandRoute) -> String {
        switch route {
        case .attachClipboard: "Attach clipboard to project"
        case .attachSelection: "Attach selection to project"
        default: "Attach draft to project"
        }
    }

    private static func activityRow(
        sequence: UInt64,
        entry: WorkbenchActivityEntry,
        index: Int,
        commandID: WorkbenchCommandID
    ) -> ResultItem {
        let subtitle: String
        let action: Action
        if entry.isResumableDraft {
            subtitle = entry.preview ?? entry.detail ?? "Press Return to resume"
            let payload = (try? ModuleActionCoding.encode(
                WorkbenchCaptureAction.resumeActivity(entryID: entry.id)
            )) ?? Data()
            action = Action(
                id: ActionID(module: entry.moduleID, key: "command.activity.\(entry.id.uuidString)"),
                title: entry.title,
                kind: .custom(payload: payload, handler: .workbench)
            )
        } else {
            subtitle = entry.preview ?? entry.detail ?? "Recorded activity"
            action = Action(
                id: ActionID(module: .workbench, key: "command.activity.\(entry.id.uuidString)"),
                title: entry.title,
                kind: .noop
            )
        }
        return ResultItem(
            id: ResultID(module: .workbench, key: "command.activity.\(entry.id.uuidString)"),
            title: entry.title,
            titleAttributed: AttributedString(entry.title),
            subtitle: subtitle,
            icon: .symbol(entry.isResumableDraft ? "clock.arrow.circlepath" : "clock"),
            primaryAction: action,
            rankingHints: RankingHints(basePriority: 94 - index),
            rowKind: .starter
        )
    }

    private static func commandRow(
        sequence: UInt64,
        title: String,
        subtitle: String,
        commandID: WorkbenchCommandID
    ) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(WorkbenchCommandAction.execute(commandID))) ?? Data()
        return ResultItem(
            id: ResultID(module: .workbench, key: "command.\(commandID.rawValue)"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle.isEmpty ? "Press Return to run" : subtitle,
            icon: .symbol("arrow.right.circle"),
            primaryAction: Action(
                id: ActionID(module: .workbench, key: "command.\(commandID.rawValue)"),
                title: title,
                kind: .custom(payload: payload, handler: .workbench)
            ),
            rankingHints: RankingHints(basePriority: 96),
            rowKind: .starter
        )
    }

    private static func disabledRow(sequence: UInt64, title: String) -> ResultItem {
        ResultItem(
            id: ResultID(module: .workbench, key: "command.disabled"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: "Module disabled in Settings",
            icon: .symbol("nosign"),
            primaryAction: Action(
                id: ActionID(module: .workbench, key: "command.disabled"),
                title: title,
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 0),
            rowKind: .starter
        )
    }

    private static func previewSubtitle(
        for definition: WorkbenchCommandDefinition,
        context: WorkbenchContext
    ) -> String {
        guard let source = definition.captureSource else { return "" }
        switch source {
        case .selection:
            return String((context.selectionText ?? "").prefix(48))
        case .clipboardText, .clipboardURL:
            return String((context.clipboardPreview ?? "").prefix(48))
        case .projectContext:
            return context.currentProject?.projectLabel ?? ""
        }
    }
}
