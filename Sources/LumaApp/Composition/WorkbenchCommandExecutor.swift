import Foundation
import LumaCore
import LumaModules

/// Executes workbench command routes without global search fan-out.
struct WorkbenchCommandExecutor {
    private let captureService = DefaultWorkbenchCaptureService()
    private let contextBuilder = WorkbenchContextBuilder()

    func handle(
        route: WorkbenchCommandRoute,
        enabledModuleIDs: Set<ModuleIdentifier>,
        pinnedModuleIDs: Set<ModuleIdentifier>,
        clipboardPreview: String?,
        selectionText: String?
    ) async -> WorkbenchCommandOutcome {
        let context = await contextBuilder.build(
            enabledModuleIDs: enabledModuleIDs,
            pinnedModuleIDs: pinnedModuleIDs,
            clipboardPreview: clipboardPreview,
            selectionText: selectionText
        )

        switch route {
        case .none:
            return .notHandled
        case .continueProject, .projectWork, .projectOpen:
            return continueProject(context: context)
        case .attachProject, .attachClipboard, .attachSelection:
            return await attachProject(
                context: context,
                source: attachSource(for: route)
            )
        case .projectRecent:
            return projectRecent(context: context)
        case .projectResume:
            return projectResume(context: context)
        case .projectLinks:
            return projectLinks(context: context)
        case .projectCapture:
            return await projectCapture(context: context)
        case .capture(let definition):
            return await executeCapture(definition: definition, context: context)
        }
    }

    private func attachSource(for route: WorkbenchCommandRoute) -> WorkbenchCaptureSource {
        switch route {
        case .attachClipboard: .clipboardText
        case .attachSelection: .selection
        default: .projectContext
        }
    }

    private func continueProject(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), let project = context.currentProject else {
            return .status("No active project context")
        }
        let payload = (try? ModuleActionCoding.encode(ProjectAction.openCurrentDetail(project))) ?? Data()
        return .openDetail(.projects, payload: payload)
    }

    private func projectRecent(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status("No active project context")
        }
        let recent = context.activitySnapshot.enabledCurrentProjectRecent(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        if let top = recent.first, top.isResumableDraft {
            return .resumeActivity(top.id)
        }
        return continueProject(context: context)
    }

    private func projectResume(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status("No active project context")
        }
        let drafts = context.activitySnapshot.enabledCurrentProjectDrafts(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        if let draft = drafts.first {
            return .resumeActivity(draft.id)
        }
        return continueProject(context: context)
    }

    private func projectLinks(context: WorkbenchContext) -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status("No active project context")
        }
        let links = context.linkSnapshot.enabledLinks(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        if let link = links.first {
            return .openLinked(link.id)
        }
        return continueProject(context: context)
    }

    private func projectCapture(context: WorkbenchContext) async -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchProjects), context.currentProject != nil else {
            return .status("No active project context")
        }
        let targets: [(WorkbenchCaptureTarget, ModuleIdentifier)] = [
            (.projectSnippetDraft, .workbenchSnippets),
            (.quicklinkDraft, .workbenchQuicklinks),
            (.todoDraft, .workbenchTodo),
            (.noteDraft, .workbenchNotes)
        ]
        guard let first = targets.first(where: { context.isEnabled($0.1) }) else {
            return .status("Enable capture modules in Settings")
        }
        return await executeProjectCapture(target: first.0, context: context)
    }

    private func attachProject(
        context: WorkbenchContext,
        source: WorkbenchCaptureSource
    ) async -> WorkbenchCommandOutcome {
        guard context.isEnabled(.workbenchSnippets), context.currentProject != nil else {
            return .status("Enable Snippets and open a project context first")
        }
        return await executeProjectCapture(target: .projectSnippetDraft, context: context, source: source)
    }

    private func executeProjectCapture(
        target: WorkbenchCaptureTarget,
        context: WorkbenchContext,
        source: WorkbenchCaptureSource = .projectContext
    ) async -> WorkbenchCommandOutcome {
        guard let result = await captureService.capture(
            source: source,
            target: target,
            context: context
        ) else {
            return .status("Could not capture to project")
        }
        await captureService.applyResult(
            result,
            context: context,
            attribution: WorkbenchCaptureAttribution(sourceKind: .command, followUp: followUp(for: target))
        )
        await MainActor.run {
            captureService.stagePendingState(for: result)
        }
        switch target {
        case .noteDraft:
            if let payload = result.actionPayload {
                return .runAction(.custom(payload: payload, handler: .notes))
            }
            return .status("Note capture failed")
        case .todoDraft:
            return .replaceQuery(TodoModule.resumeQuery(forCapture: result.preview))
        case .snippetDraft, .quicklinkDraft, .projectSnippetDraft:
            return .openDetail(result.moduleID, payload: result.openDetailPayload)
        }
    }

    private func executeCapture(
        definition: WorkbenchCommandDefinition,
        context: WorkbenchContext
    ) async -> WorkbenchCommandOutcome {
        guard context.isEnabled(definition.requiredModule) else {
            return .status("Module disabled in Settings")
        }
        guard let source = definition.captureSource,
              let target = definition.captureTarget else {
            return .notHandled
        }
        guard let result = await captureService.capture(source: source, target: target, context: context) else {
            return .status("Nothing to capture")
        }
        await captureService.applyResult(
            result,
            context: context,
            attribution: WorkbenchCaptureAttribution(
                sourceKind: .command,
                followUp: followUp(for: target)
            )
        )
        await MainActor.run {
            captureService.stagePendingState(for: result)
        }

        switch target {
        case .noteDraft:
            if let payload = result.actionPayload {
                return .runAction(.custom(payload: payload, handler: .notes))
            }
            return .status("Note capture failed")
        case .todoDraft:
            return .replaceQuery(TodoModule.resumeQuery(forCapture: result.preview))
        case .snippetDraft, .quicklinkDraft, .projectSnippetDraft:
            return .openDetail(result.moduleID, payload: result.openDetailPayload)
        }
    }

    private func followUp(for target: WorkbenchCaptureTarget) -> WorkbenchCaptureFollowUp {
        switch target {
        case .noteDraft: .runNotesAction
        case .todoDraft: .replaceQuery
        case .snippetDraft, .quicklinkDraft, .projectSnippetDraft: .openDetail
        }
    }

    func handle(
        commandID: WorkbenchCommandID,
        enabledModuleIDs: Set<ModuleIdentifier>,
        pinnedModuleIDs: Set<ModuleIdentifier>,
        clipboardPreview: String?,
        selectionText: String?
    ) async -> WorkbenchCommandOutcome {
        switch commandID {
        case .continueProject, .projectWork, .projectOpen:
            return await handle(
                route: commandID == .projectOpen ? .projectOpen : .projectWork,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .attachProject, .attachClipboard, .attachSelection:
            let route: WorkbenchCommandRoute = switch commandID {
            case .attachClipboard: .attachClipboard
            case .attachSelection: .attachSelection
            default: .attachProject
            }
            return await handle(
                route: route,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .projectRecent:
            return await handle(
                route: .projectRecent,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .projectLinks:
            return await handle(
                route: .projectLinks,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .projectResume:
            return await handle(
                route: .projectResume,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .projectCapture:
            return await handle(
                route: .projectCapture,
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        case .captureClipboardNote, .captureClipboardTodo, .captureClipboardQuicklink,
             .captureSelectionNote, .captureSelectionTodo,
             .projectNote, .projectTodo:
            guard let definition = WorkbenchCommandRouter().definition(for: commandID) else {
                return .notHandled
            }
            return await handle(
                route: .capture(definition),
                enabledModuleIDs: enabledModuleIDs,
                pinnedModuleIDs: pinnedModuleIDs,
                clipboardPreview: clipboardPreview,
                selectionText: selectionText
            )
        }
    }
}

enum WorkbenchCommandOutcome: Sendable {
    case notHandled
    case status(String)
    case openDetail(ModuleIdentifier, payload: Data?)
    case replaceQuery(String)
    case runAction(ActionKind)
    case resumeActivity(UUID)
    case openLinked(UUID)
}
