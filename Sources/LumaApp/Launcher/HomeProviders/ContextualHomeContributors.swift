import Foundation
import LumaCore
import LumaModules
import LumaServices

struct ProjectHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard let project = context.workbench?.currentProject else { return [] }

        var rows: [HomeContribution] = []
        if context.isEnabled(.projects), let row = currentProjectRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.current", kind: .continueFlow, basePriority: 88))
        }
        if context.isHot(.snippets), let row = projectSnippetRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-snippet", kind: .create, basePriority: 80))
        }
        if context.isHot(.quicklinks), let row = projectQuicklinkRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-quicklink", kind: .create, basePriority: 79))
        }
        if context.isHot(.todo), let row = projectTodoRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-todo", kind: .create, basePriority: 77))
        }
        if context.isHot(.notes), let row = projectNoteRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-note", kind: .create, basePriority: 76))
        }
        if context.isHot(.commands), let row = projectCommandsRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-commands", kind: .create, basePriority: 78))
        }
        return rows
    }

    private func projectSnippetRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let draft = ProjectContextSuggestions.snippetDraft(for: context)
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.project-snippet"),
            title: "New snippet for \(name)",
            titleAttributed: AttributedString("New snippet for \(name)"),
            subtitle: draft.trigger,
            icon: .symbol("text.badge.plus"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-snippet",
                title: CrossModuleActionTitles.newProjectSnippet,
                source: .projectContext,
                target: .projectSnippetDraft,
                moduleID: .snippets
            ),
            rankingHints: RankingHints(basePriority: 80),
            rowKind: .starter
        )
    }

    private func projectQuicklinkRow(context: CurrentProjectContext) -> ResultItem? {
        guard let draft = ProjectQuicklinkDraftSource(context: context).quicklinkDraft() else { return nil }
        return ResultItem(
            id: ResultID(module: .quicklinks, key: "contextual.project-quicklink"),
            title: "Quicklink to \(draft.name)",
            titleAttributed: AttributedString("Quicklink to \(draft.name)"),
            subtitle: draft.trigger,
            icon: .symbol("link.badge.plus"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-quicklink",
                title: CrossModuleActionTitles.projectFolderQuicklink,
                source: .projectContext,
                target: .quicklinkDraft,
                moduleID: .quicklinks
            ),
            rankingHints: RankingHints(basePriority: 79),
            rowKind: .starter
        )
    }

    private func projectCommandsRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let payload = (try? ModuleActionCoding.encode(CommandsAction.revealConfig)) ?? Data()
        return ResultItem(
            id: ResultID(module: .commands, key: "contextual.project-commands"),
            title: "Script commands for \(name)",
            titleAttributed: AttributedString("Script commands for \(name)"),
            subtitle: "Reveal commands.json in Finder",
            icon: .symbol("terminal"),
            primaryAction: Action(
                id: ActionID(module: .commands, key: "contextual.project-commands"),
                title: CrossModuleActionTitles.editScriptCommands,
                kind: .custom(payload: payload, handler: .commands)
            ),
            rankingHints: RankingHints(basePriority: 78),
            rowKind: .starter
        )
    }

    private func projectTodoRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let capture = ProjectContextSuggestions.todoDraft(for: context)
        return ResultItem(
            id: ResultID(module: .todo, key: "contextual.project-todo"),
            title: "Capture todo for \(name)",
            titleAttributed: AttributedString("Capture todo for \(name)"),
            subtitle: capture,
            icon: .symbol("checklist"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-todo",
                title: CrossModuleActionTitles.openTodo,
                source: .projectContext,
                target: .todoDraft,
                moduleID: .todo
            ),
            rankingHints: RankingHints(basePriority: 77),
            rowKind: .starter
        )
    }

    private func projectNoteRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let text = ProjectContextSuggestions.noteCaptureText(for: context)
        return ResultItem(
            id: ResultID(module: .notes, key: "contextual.project-note"),
            title: "Capture note for \(name)",
            titleAttributed: AttributedString("Capture note for \(name)"),
            subtitle: String(text.prefix(48)),
            icon: .symbol("square.and.pencil"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-note",
                title: CrossModuleActionTitles.appendToNote,
                source: .projectContext,
                target: .noteDraft,
                moduleID: .notes
            ),
            rankingHints: RankingHints(basePriority: 76),
            rowKind: .starter
        )
    }

    private func currentProjectRow(context: CurrentProjectContext) -> ResultItem? {
        let title = "In \(context.frontAppName): \(context.projectLabel)"
        let subtitle = context.filename ?? context.matchedProjectPath ?? context.projectLabel
        let payload = (try? ModuleActionCoding.encode(ProjectAction.openCurrentDetail(context))) ?? Data()
        var secondaries: [Action] = []
        if let path = context.matchedProjectPath {
            let name = context.projectName ?? context.projectLabel
            let notesPayload = (try? ModuleActionCoding.encode(ProjectAction.openNotes(path: path, projectName: name))) ?? Data()
            secondaries.append(Action(
                id: ActionID(module: .projects, key: "contextual.notes"),
                title: CrossModuleActionTitles.openNotesForProject,
                kind: .custom(payload: notesPayload, handler: .projects)
            ))
        }
        return ResultItem(
            id: ResultID(module: .projects, key: "contextual.current"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .symbol("folder"),
            primaryAction: Action(
                id: ActionID(module: .projects, key: "contextual.current"),
                title: "Current Project",
                kind: .openModuleDetail(.projects, payload: payload)
            ),
            secondaryActions: secondaries,
            rankingHints: RankingHints(basePriority: 88),
            rowKind: .starter
        )
    }
}

struct ProjectActivityHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard let workbench = context.workbench,
              let project = workbench.currentProject,
              context.isEnabled(.projects) else { return [] }

        var rows: [HomeContribution] = []

        if let row = continueWorkspaceRow(context: project) {
            rows.append(HomeContribution(
                item: row,
                key: "contextual.project-workspace",
                kind: .continueFlow,
                basePriority: 87
            ))
        }

        let drafts = workbench.activitySnapshot.enabledCurrentProjectDrafts(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1
        )
        for draft in drafts {
            rows.append(HomeContribution(
                item: recentDraftRow(entry: draft),
                key: "contextual.project-activity.\(draft.id.uuidString)",
                kind: .continueFlow,
                basePriority: 85
            ))
        }

        let linked = workbench.linkSnapshot.enabledLinks(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 2
        )
        for link in linked {
            rows.append(HomeContribution(
                item: linkedItemRow(link: link),
                key: "contextual.project-link.\(link.id.uuidString)",
                kind: .continueFlow,
                basePriority: 83
            ))
        }

        let activities = workbench.activitySnapshot.enabledCurrentProjectRecent(
            enabledModuleIDs: context.enabledModuleIDs,
            limit: 1,
            excludingResumableDrafts: true
        )
        if let latest = activities.first {
            rows.append(HomeContribution(
                item: recentActivityRow(entry: latest),
                key: "contextual.project-activity.recent.\(latest.id.uuidString)",
                kind: .continueFlow,
                basePriority: 84
            ))
        }

        if context.isHot(.snippets), let row = attachClipboardRow(context: project) {
            rows.append(HomeContribution(
                item: row,
                key: "contextual.project-attach-clip",
                kind: .create,
                basePriority: 78
            ))
        }
        if context.isHot(.snippets), let row = attachSelectionRow(context: project) {
            rows.append(HomeContribution(
                item: row,
                key: "contextual.project-attach-sel",
                kind: .create,
                basePriority: 77
            ))
        }

        return rows
    }

    private func continueWorkspaceRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let payload = (try? ModuleActionCoding.encode(ProjectAction.openCurrentDetail(context))) ?? Data()
        return ResultItem(
            id: ResultID(module: .projects, key: "contextual.project-workspace"),
            title: "Continue project workspace",
            titleAttributed: AttributedString("Continue project workspace"),
            subtitle: name,
            icon: .symbol("arrow.triangle.branch"),
            primaryAction: Action(
                id: ActionID(module: .projects, key: "contextual.project-workspace"),
                title: "Continue project workspace",
                kind: .openModuleDetail(.projects, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: 87),
            rowKind: .starter
        )
    }

    private func recentDraftRow(entry: WorkbenchActivityEntry) -> ResultItem {
        ResultItem(
            id: ResultID(module: entry.moduleID, key: "contextual.project-activity.\(entry.id.uuidString)"),
            title: entry.title,
            titleAttributed: AttributedString(entry.title),
            subtitle: entry.preview ?? entry.detail ?? "Resume draft",
            icon: .symbol("doc.badge.clock"),
            primaryAction: WorkbenchHomeCaptureRows.resumeAction(entry: entry),
            rankingHints: RankingHints(basePriority: 85),
            rowKind: .starter
        )
    }

    private func linkedItemRow(link: WorkbenchProjectLink) -> ResultItem {
        let ref = link.entityRef
        let action = WorkbenchHomeCaptureRows.openLinkedAction(link: link)
        return ResultItem(
            id: ResultID(module: ref.moduleID, key: "contextual.project-link.\(link.id.uuidString)"),
            title: "Review linked: \(ref.title)",
            titleAttributed: AttributedString("Review linked: \(ref.title)"),
            subtitle: ref.subtitle ?? "Project linked item",
            icon: .symbol("link"),
            primaryAction: action,
            rankingHints: RankingHints(basePriority: 83),
            rowKind: .starter
        )
    }

    private func recentActivityRow(entry: WorkbenchActivityEntry) -> ResultItem {
        ResultItem(
            id: ResultID(module: entry.moduleID, key: "contextual.project-activity.recent.\(entry.id.uuidString)"),
            title: entry.title,
            titleAttributed: AttributedString(entry.title),
            subtitle: entry.preview ?? entry.detail ?? entry.sourceKind?.rawValue ?? "",
            icon: .symbol("clock"),
            primaryAction: Action(
                id: ActionID(module: entry.moduleID, key: "contextual.project-activity.recent"),
                title: entry.title,
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 84),
            rowKind: .starter
        )
    }

    private func attachClipboardRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.project-attach-clip"),
            title: "Attach clipboard to \(name)",
            titleAttributed: AttributedString("Attach clipboard to \(name)"),
            subtitle: "Project snippet draft",
            icon: .symbol("paperclip"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-attach-clip",
                title: "Attach clipboard to project",
                source: .clipboardText,
                target: .projectSnippetDraft,
                moduleID: .snippets
            ),
            rankingHints: RankingHints(basePriority: 78),
            rowKind: .starter
        )
    }

    private func attachSelectionRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.project-attach-sel"),
            title: "Attach selection to \(name)",
            titleAttributed: AttributedString("Attach selection to \(name)"),
            subtitle: "Project snippet draft",
            icon: .symbol("selection.pin.in.out"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.project-attach-sel",
                title: "Attach selection to project",
                source: .selection,
                target: .projectSnippetDraft,
                moduleID: .snippets
            ),
            rankingHints: RankingHints(basePriority: 77),
            rowKind: .starter
        )
    }
}

struct SelectionHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard context.isEnabled(.translate) else { return [] }
        guard let text = context.workbench?.selectionText,
              !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              TranslationUserMessages.shouldTranslate(text) else { return [] }
        return [
            HomeContribution(
                item: translateSelectedRow(text: text),
                key: "contextual.translate",
                kind: .utility,
                basePriority: 90
            )
        ]
    }

    private func translateSelectedRow(text: String) -> ResultItem {
        let preview = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let subtitle = String(preview.prefix(48))
        return ResultItem(
            id: ResultID(module: .translate, key: "contextual.translate"),
            title: "Translate selected text",
            titleAttributed: AttributedString("Translate selected text"),
            subtitle: subtitle,
            icon: .symbol("character.bubble"),
            primaryAction: Action(
                id: ActionID(module: .translate, key: "contextual.translate"),
                title: "Translate",
                kind: .translateText(preview)
            ),
            rankingHints: RankingHints(basePriority: 90)
        )
    }
}

struct ClipboardHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard context.isEnabled(.clipboard) else { return [] }
        guard let preview = context.workbench?.clipboardPreview,
              !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return [] }

        var rows: [HomeContribution] = []
        if let row = clipboardTransformRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: row.id.key, kind: .transform, basePriority: 84))
        }
        if context.isHot(.notes), let row = saveClipboardToNoteRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.clip-note", kind: .create, basePriority: 83))
        }
        if context.isHot(.todo), let row = saveClipboardAsTodoRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.clip-todo", kind: .create, basePriority: 82))
        }
        if context.isHot(.snippets), let row = saveClipboardAsSnippetRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.clip-snippet", kind: .create, basePriority: 81))
        }
        if context.isHot(.quicklinks), let row = saveURLAsQuicklinkRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.url-quicklink", kind: .create, basePriority: 80))
        }
        return rows
    }

    private func clipboardTransformRow(preview: String) -> ResultItem? {
        let kind = ClipboardTextOps.classify(preview)
        if kind == .json, let json = ClipboardTextOps.detectJSON(preview) {
            return transformRow(
                key: "format-json",
                title: CrossModuleActionTitles.formatJSON,
                subtitle: String(preview.prefix(48)),
                output: json
            )
        }
        if let decoded = ClipboardTextOps.decodeBase64(preview) {
            return transformRow(
                key: "decode-base64",
                title: CrossModuleActionTitles.decodeBase64,
                subtitle: String(preview.prefix(48)),
                output: decoded
            )
        }
        return nil
    }

    private func transformRow(key: String, title: String, subtitle: String, output: String) -> ResultItem {
        ResultItem(
            id: ResultID(module: .clipboard, key: "contextual.\(key)"),
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .symbol("wand.and.stars"),
            primaryAction: Action(
                id: ActionID(module: .clipboard, key: "contextual.\(key)"),
                title: title,
                kind: .copyToPasteboard(output)
            ),
            rankingHints: RankingHints(basePriority: 84),
            rowKind: .starter
        )
    }

    private func saveClipboardToNoteRow(preview: String) -> ResultItem? {
        guard ClipboardTextOps.classify(preview) != .url else { return nil }
        let text = preview.trimmingCharacters(in: .whitespacesAndNewlines)
        return ResultItem(
            id: ResultID(module: .notes, key: "contextual.clip-note"),
            title: "Append clipboard to daily note",
            titleAttributed: AttributedString("Append clipboard to daily note"),
            subtitle: String(text.prefix(48)),
            icon: .symbol("square.and.pencil"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.clip-note",
                title: CrossModuleActionTitles.appendToNote,
                source: .clipboardText,
                target: .noteDraft,
                moduleID: .notes
            ),
            rankingHints: RankingHints(basePriority: 83),
            rowKind: .starter
        )
    }

    private func saveClipboardAsTodoRow(preview: String) -> ResultItem? {
        guard ClipboardTextOps.classify(preview) != .url else { return nil }
        let text = WorkbenchCaptureDraftBuilder.buildTodoCaptureText(preview)
        guard !text.isEmpty else { return nil }
        return ResultItem(
            id: ResultID(module: .todo, key: "contextual.clip-todo"),
            title: "Save clipboard as todo",
            titleAttributed: AttributedString("Save clipboard as todo"),
            subtitle: String(text.prefix(48)),
            icon: .symbol("checklist"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.clip-todo",
                title: CrossModuleActionTitles.openTodo,
                source: .clipboardText,
                target: .todoDraft,
                moduleID: .todo
            ),
            rankingHints: RankingHints(basePriority: 82),
            rowKind: .starter
        )
    }

    private func saveClipboardAsSnippetRow(preview: String) -> ResultItem? {
        guard preview.count >= 8, ClipboardTextOps.classify(preview) != .url else { return nil }
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.clip-snippet"),
            title: "Save clipboard as snippet",
            titleAttributed: AttributedString("Save clipboard as snippet"),
            subtitle: String(preview.prefix(48)),
            icon: .symbol("text.badge.plus"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.clip-snippet",
                title: CrossModuleActionTitles.createSnippet,
                source: .clipboardText,
                target: .snippetDraft,
                moduleID: .snippets
            ),
            rankingHints: RankingHints(basePriority: 81),
            rowKind: .starter
        )
    }

    private func saveURLAsQuicklinkRow(preview: String) -> ResultItem? {
        guard let draft = WorkbenchCaptureDraftBuilder.buildQuicklinkDraft(text: preview),
              let url = URL(string: draft.urlTemplate) else { return nil }
        return ResultItem(
            id: ResultID(module: .quicklinks, key: "contextual.url-quicklink"),
            title: "Save URL as Quicklink",
            titleAttributed: AttributedString("Save URL as Quicklink"),
            subtitle: url.host ?? url.absoluteString,
            icon: .symbol("link.badge.plus"),
            primaryAction: WorkbenchHomeCaptureRows.captureAction(
                key: "contextual.url-quicklink",
                title: CrossModuleActionTitles.addQuicklink,
                source: .clipboardURL,
                target: .quicklinkDraft,
                moduleID: .quicklinks
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: .quicklinks, key: "contextual.copy-url"),
                    title: CrossModuleActionTitles.copyURL,
                    kind: .copyToPasteboard(url.absoluteString)
                )
            ],
            rankingHints: RankingHints(basePriority: 81),
            rowKind: .starter
        )
    }
}

struct ContinueHomeContributor: HomeContributor {
    let notes: (any NotesContinueClient)?
    let todo: (any TodoContinueClient)?
    let media: (any MediaContinueClient)?
    let wordbook: (any WordbookContinueClient)?

    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        async let dailyRow = context.isHot(.notes) ? continueDailyNoteRow() : nil
        async let todoRow = context.isHot(.todo) ? topTodoRow() : nil
        async let recordsRow = context.isHot(.media) ? continueRecordsRow() : nil
        async let wordbookRow = context.isHot(.wordbook) ? continueWordbookRow() : nil

        let rows = await (dailyRow, todoRow, recordsRow, wordbookRow)
        return [
            rows.0.map { HomeContribution(item: $0, key: "contextual.daily", kind: .continueFlow, basePriority: 87) },
            rows.1.map { HomeContribution(item: $0, key: "contextual.todo", kind: .continueFlow, basePriority: 86) },
            rows.2.map { HomeContribution(item: $0, key: "contextual.records", kind: .continueFlow, basePriority: 75) },
            rows.3.map { HomeContribution(item: $0, key: "contextual.wordbook", kind: .continueFlow, basePriority: 74) }
        ].compactMap { $0 }
    }

    private func continueDailyNoteRow() async -> ResultItem? {
        guard let notes,
              let path = await notes.dailyNotePath() else { return nil }
        let name = URL(fileURLWithPath: path).deletingPathExtension().lastPathComponent
        let payload = (try? ModuleActionCoding.encode(NotesAction.open(path: path))) ?? Data()
        return ResultItem(
            id: ResultID(module: .notes, key: "contextual.daily"),
            title: "Continue daily note",
            titleAttributed: AttributedString("Continue daily note"),
            subtitle: name,
            icon: .symbol("calendar"),
            primaryAction: Action(
                id: ActionID(module: .notes, key: "contextual.daily"),
                title: CrossModuleActionTitles.openDailyNote,
                kind: .custom(payload: payload, handler: .notes)
            ),
            rankingHints: RankingHints(basePriority: 87),
            rowKind: .starter
        )
    }

    private func topTodoRow() async -> ResultItem? {
        guard let todo else { return nil }
        guard let reminder = try? await todo.firstTodayDueReminder() else { return nil }
        let completePayload = (try? ModuleActionCoding.encode(TodoAction.complete(id: reminder.id))) ?? Data()
        return ResultItem(
            id: ResultID(module: .todo, key: "contextual.todo"),
            title: reminder.title,
            titleAttributed: AttributedString(reminder.title),
            subtitle: "Due today",
            icon: .symbol("checkmark.circle"),
            primaryAction: Action(
                id: ActionID(module: .todo, key: "contextual.open"),
                title: CrossModuleActionTitles.openTodo,
                kind: .openModuleDetail(.todo, payload: nil)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: .todo, key: "contextual.complete"),
                    title: CrossModuleActionTitles.markComplete,
                    kind: .custom(payload: completePayload, handler: .todo)
                )
            ],
            rankingHints: RankingHints(basePriority: 86),
            rowKind: .starter
        )
    }

    private func continueRecordsRow() async -> ResultItem? {
        guard let media else { return nil }
        let count = await media.inProgressCount()
        guard count > 0 else { return nil }
        let subtitle = count == 1 ? "1 in progress" : "\(count) in progress"
        return ResultItem(
            id: ResultID(module: .media, key: "contextual.records"),
            title: "Continue Records",
            titleAttributed: AttributedString("Continue Records"),
            subtitle: subtitle,
            icon: .symbol("books.vertical"),
            primaryAction: Action(
                id: ActionID(module: .media, key: "contextual.open"),
                title: "Open Records",
                kind: .openModuleDetail(.media, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: 75),
            rowKind: .starter
        )
    }

    private func continueWordbookRow() async -> ResultItem? {
        guard let wordbook else { return nil }
        let due = await wordbook.dueTodayCount()
        guard due > 0 else { return nil }
        let subtitle = due == 1 ? "1 due today" : "\(due) due today"
        let payload = (try? ModuleActionCoding.encode(WordbookAction.review)) ?? Data()
        return ResultItem(
            id: ResultID(module: .wordbook, key: "contextual.wordbook"),
            title: "Continue Wordbook",
            titleAttributed: AttributedString("Continue Wordbook"),
            subtitle: subtitle,
            icon: .symbol("text.book.closed"),
            primaryAction: Action(
                id: ActionID(module: .wordbook, key: "contextual.open"),
                title: CrossModuleActionTitles.continueWordbook,
                kind: .openModuleDetail(.wordbook, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: 74),
            rowKind: .starter
        )
    }
}
