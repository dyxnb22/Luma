import Foundation
import LumaCore
import LumaModules
import LumaServices

struct ProjectHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard let project = await CurrentProjectService.shared.snapshot() else { return [] }

        var rows: [HomeContribution] = []
        if context.enabledModuleIDs.contains(.projects), let row = currentProjectRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.current", kind: .continueFlow, basePriority: 88))
        }
        if isHot(.snippets, context: context), let row = projectSnippetRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-snippet", kind: .create, basePriority: 80))
        }
        if isHot(.quicklinks, context: context), let row = projectQuicklinkRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-quicklink", kind: .create, basePriority: 79))
        }
        if isHot(.commands, context: context), let row = projectCommandsRow(context: project) {
            rows.append(HomeContribution(item: row, key: "contextual.project-commands", kind: .create, basePriority: 78))
        }
        return rows
    }

    private func isHot(_ id: ModuleIdentifier, context: HomeContributionContext) -> Bool {
        context.enabledModuleIDs.contains(id) && context.pinnedModuleIDs.contains(id)
    }

    private func projectSnippetRow(context: CurrentProjectContext) -> ResultItem? {
        let name = context.projectName ?? context.projectLabel
        let draft = ProjectContextSuggestions.snippetDraft(for: context)
        let payload = (try? ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))) ?? Data()
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.project-snippet"),
            title: "New snippet for \(name)",
            titleAttributed: AttributedString("New snippet for \(name)"),
            subtitle: draft.trigger,
            icon: .symbol("text.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: .snippets, key: "contextual.project-snippet"),
                title: CrossModuleActionTitles.newProjectSnippet,
                kind: .openModuleDetail(.snippets, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: 80),
            rowKind: .starter
        )
    }

    private func projectQuicklinkRow(context: CurrentProjectContext) -> ResultItem? {
        guard let draft = ProjectQuicklinkDraftSource(context: context).quicklinkDraft() else { return nil }
        let payload = (try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))) ?? Data()
        return ResultItem(
            id: ResultID(module: .quicklinks, key: "contextual.project-quicklink"),
            title: "Quicklink to \(draft.name)",
            titleAttributed: AttributedString("Quicklink to \(draft.name)"),
            subtitle: draft.trigger,
            icon: .symbol("link.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: .quicklinks, key: "contextual.project-quicklink"),
                title: CrossModuleActionTitles.projectFolderQuicklink,
                kind: .openModuleDetail(.quicklinks, payload: payload)
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

struct SelectionHomeContributor: HomeContributor {
    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        guard context.enabledModuleIDs.contains(.translate) else { return [] }
        guard let text = await SelectionSnapshotService.shared.snapshot(),
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
        guard context.enabledModuleIDs.contains(.clipboard) else { return [] }
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return [] }

        var rows: [HomeContribution] = []
        if let row = clipboardTransformRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: row.id.key, kind: .transform, basePriority: 84))
        }
        if isHot(.notes, context: context), let row = saveClipboardToNoteRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.clip-note", kind: .create, basePriority: 83))
        }
        if isHot(.snippets, context: context), let row = saveClipboardAsSnippetRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.clip-snippet", kind: .create, basePriority: 82))
        }
        if isHot(.quicklinks, context: context), let row = saveURLAsQuicklinkRow(preview: preview) {
            rows.append(HomeContribution(item: row, key: "contextual.url-quicklink", kind: .create, basePriority: 81))
        }
        return rows
    }

    private func isHot(_ id: ModuleIdentifier, context: HomeContributionContext) -> Bool {
        context.enabledModuleIDs.contains(id) && context.pinnedModuleIDs.contains(id)
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
        let payload = (try? ModuleActionCoding.encode(NotesAction.captureToDaily(text: text))) ?? Data()
        return ResultItem(
            id: ResultID(module: .notes, key: "contextual.clip-note"),
            title: "Append clipboard to daily note",
            titleAttributed: AttributedString("Append clipboard to daily note"),
            subtitle: String(text.prefix(48)),
            icon: .symbol("square.and.pencil"),
            primaryAction: Action(
                id: ActionID(module: .notes, key: "contextual.clip-note"),
                title: CrossModuleActionTitles.appendToNote,
                kind: .custom(payload: payload, handler: .notes)
            ),
            rankingHints: RankingHints(basePriority: 83),
            rowKind: .starter
        )
    }

    private func saveClipboardAsSnippetRow(preview: String) -> ResultItem? {
        guard preview.count >= 8, ClipboardTextOps.classify(preview) != .url else { return nil }
        let draft = SnippetDraft.fromClipboard(preview)
        let payload = (try? ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))) ?? Data()
        return ResultItem(
            id: ResultID(module: .snippets, key: "contextual.clip-snippet"),
            title: "Save clipboard as snippet",
            titleAttributed: AttributedString("Save clipboard as snippet"),
            subtitle: String(preview.prefix(48)),
            icon: .symbol("text.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: .snippets, key: "contextual.clip-snippet"),
                title: CrossModuleActionTitles.createSnippet,
                kind: .openModuleDetail(.snippets, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: 82),
            rowKind: .starter
        )
    }

    private func saveURLAsQuicklinkRow(preview: String) -> ResultItem? {
        guard let draft = TextQuicklinkDraftSource(text: preview).quicklinkDraft(),
              let url = URL(string: draft.urlTemplate) else { return nil }
        let payload = (try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))) ?? Data()
        return ResultItem(
            id: ResultID(module: .quicklinks, key: "contextual.url-quicklink"),
            title: "Save URL as Quicklink",
            titleAttributed: AttributedString("Save URL as Quicklink"),
            subtitle: url.host ?? url.absoluteString,
            icon: .symbol("link.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: .quicklinks, key: "contextual.url-quicklink"),
                title: CrossModuleActionTitles.addQuicklink,
                kind: .openModuleDetail(.quicklinks, payload: payload)
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
    let notesModule: NotesModule?
    let todoModule: TodoModule?
    let mediaModule: MediaModule?
    let wordbookModule: WordbookModule?

    func contribute(context: HomeContributionContext) async -> [HomeContribution] {
        async let dailyRow = isHot(.notes, context: context) ? continueDailyNoteRow() : nil
        async let todoRow = isHot(.todo, context: context) ? topTodoRow() : nil
        async let recordsRow = isHot(.media, context: context) ? continueRecordsRow() : nil
        async let wordbookRow = isHot(.wordbook, context: context) ? continueWordbookRow() : nil

        let rows = await (dailyRow, todoRow, recordsRow, wordbookRow)
        return [
            rows.0.map { HomeContribution(item: $0, key: "contextual.daily", kind: .continueFlow, basePriority: 87) },
            rows.1.map { HomeContribution(item: $0, key: "contextual.todo", kind: .continueFlow, basePriority: 86) },
            rows.2.map { HomeContribution(item: $0, key: "contextual.records", kind: .continueFlow, basePriority: 75) },
            rows.3.map { HomeContribution(item: $0, key: "contextual.wordbook", kind: .continueFlow, basePriority: 74) }
        ].compactMap { $0 }
    }

    private func isHot(_ id: ModuleIdentifier, context: HomeContributionContext) -> Bool {
        context.enabledModuleIDs.contains(id) && context.pinnedModuleIDs.contains(id)
    }

    private func continueDailyNoteRow() async -> ResultItem? {
        guard let notesModule,
              let path = await notesModule.dailyNotePath() else { return nil }
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
        guard let todoModule else { return nil }
        guard let reminder = try? await todoModule.firstTodayDueReminder() else { return nil }
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
        guard let mediaModule else { return nil }
        let count = await mediaModule.inProgressCount()
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
        guard let wordbookModule else { return nil }
        let due = await wordbookModule.storeDueTodayCount()
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
