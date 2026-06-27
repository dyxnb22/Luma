import AppKit
import Foundation
import LumaCore
import LumaModules
import LumaServices

struct ContextualHomeProvider: LauncherHomeProvider {
    private let notesModule: NotesModule?
    private let todoModule: TodoModule?
    private let mediaModule: MediaModule?
    private let suggestionMemory: HomeSuggestionMemory

    init(
        notesModule: NotesModule? = nil,
        todoModule: TodoModule? = nil,
        mediaModule: MediaModule? = nil,
        suggestionMemory: HomeSuggestionMemory = .shared
    ) {
        self.notesModule = notesModule
        self.todoModule = todoModule
        self.mediaModule = mediaModule
        self.suggestionMemory = suggestionMemory
    }

    func items() async -> [ResultItem] {
        var suggestions: [ResultItem] = []
        let memory = suggestionMemory

        if let text = await SelectionSnapshotService.shared.snapshot(),
           !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
           TranslationUserMessages.shouldTranslate(text),
           await !memory.shouldSuppressSuggestion(key: "contextual.translate") {
            suggestions.append(translateSelectedRow(text: text))
        }

        if suggestions.count < 4,
           let projectRow = await currentProjectRow(),
           await !memory.shouldSuppressSuggestion(key: "contextual.current") {
            suggestions.append(projectRow)
        }

        if suggestions.count < 4,
           let dailyRow = await continueDailyNoteRow(),
           await !memory.shouldSuppressDailyNoteSuggestion(),
           await !memory.shouldSuppressSuggestion(key: "contextual.daily") {
            suggestions.append(dailyRow)
        }

        if suggestions.count < 4,
           let todoRow = await topTodoRow(),
           await !memory.shouldSuppressSuggestion(key: todoRow.id.key) {
            suggestions.append(todoRow)
        }

        if suggestions.count < 4,
           let transformRow = await clipboardTransformRow(),
           await !memory.shouldSuppressSuggestion(key: transformRow.id.key) {
            suggestions.append(transformRow)
        }

        if suggestions.count < 4,
           let noteRow = await saveClipboardToNoteRow(),
           await !memory.shouldSuppressSuggestion(key: "contextual.clip-note") {
            suggestions.append(noteRow)
        }

        if suggestions.count < 4,
           let snippetRow = await saveClipboardAsSnippetRow(),
           await !memory.shouldSuppressSuggestion(key: "contextual.clip-snippet") {
            suggestions.append(snippetRow)
        }

        if suggestions.count < 4,
           let quicklinkRow = await saveURLAsQuicklinkRow(),
           await !memory.shouldSuppressSuggestion(key: "contextual.url-quicklink") {
            suggestions.append(quicklinkRow)
        }

        if suggestions.count < 4,
           let recordsRow = await continueRecordsRow(),
           await !memory.shouldSuppressSuggestion(key: "contextual.records") {
            suggestions.append(recordsRow)
        }

        return Array(suggestions.prefix(4))
    }

    private func currentProjectRow() async -> ResultItem? {
        guard let context = await CurrentProjectService.shared.snapshot() else { return nil }

        let title = "In \(context.frontAppName): \(context.projectLabel)"
        let subtitle = context.filename ?? context.matchedProjectPath ?? context.projectLabel
        let payload = (try? ModuleActionCoding.encode(ProjectAction.openCurrentDetail(context))) ?? Data()

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
            rankingHints: RankingHints(basePriority: 88),
            rowKind: .starter
        )
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
                title: "Open Daily Note",
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
            id: ResultID(module: .todo, key: "contextual.open.\(reminder.id)"),
            title: reminder.title,
            titleAttributed: AttributedString(reminder.title),
            subtitle: "Due today",
            icon: .symbol("checkmark.circle"),
            primaryAction: Action(
                id: ActionID(module: .todo, key: "contextual.open"),
                title: "Open Todo",
                kind: .openModuleDetail(.todo, payload: nil)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: .todo, key: "contextual.complete"),
                    title: "Mark Complete",
                    kind: .custom(payload: completePayload, handler: .todo)
                )
            ],
            rankingHints: RankingHints(basePriority: 86),
            rowKind: .starter
        )
    }

    private func clipboardTransformRow() async -> ResultItem? {
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              !preview.isEmpty else { return nil }

        if let json = ClipboardTransform.detectJSON(preview) {
            return transformRow(
                key: "format-json",
                title: "Format JSON",
                subtitle: String(preview.prefix(48)),
                output: json
            )
        }
        if let decoded = ClipboardTransform.decodeBase64(preview) {
            return transformRow(
                key: "decode-base64",
                title: "Decode Base64",
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

    private func saveClipboardToNoteRow() async -> ResultItem? {
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              URLTextParser.firstHTTPURL(in: preview) == nil else { return nil }
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
                title: "Append to Note",
                kind: .custom(payload: payload, handler: .notes)
            ),
            rankingHints: RankingHints(basePriority: 83),
            rowKind: .starter
        )
    }

    private func saveClipboardAsSnippetRow() async -> ResultItem? {
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              preview.count >= 8,
              URLTextParser.firstHTTPURL(in: preview) == nil else { return nil }
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
                title: "Create Snippet",
                kind: .openModuleDetail(.snippets, payload: payload)
            ),
            rankingHints: RankingHints(basePriority: 82),
            rowKind: .starter
        )
    }

    private func saveURLAsQuicklinkRow() async -> ResultItem? {
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              let url = URLTextParser.firstHTTPURL(in: preview) else { return nil }
        let draft = URLQuicklinkDraft.from(url: url)
        let payload = (try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))) ?? Data()
        return ResultItem(
            id: ResultID(module: .quicklinks, key: "contextual.url-quicklink"),
            title: "Save URL as Quicklink",
            titleAttributed: AttributedString("Save URL as Quicklink"),
            subtitle: url.host ?? url.absoluteString,
            icon: .symbol("link.badge.plus"),
            primaryAction: Action(
                id: ActionID(module: .quicklinks, key: "contextual.url-quicklink"),
                title: "Add Quicklink",
                kind: .openModuleDetail(.quicklinks, payload: payload)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: .quicklinks, key: "contextual.copy-url"),
                    title: "Copy URL",
                    kind: .copyToPasteboard(url.absoluteString)
                )
            ],
            rankingHints: RankingHints(basePriority: 81),
            rowKind: .starter
        )
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
}

enum ClipboardTransform {
    static func detectJSON(_ text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix("{") || trimmed.hasPrefix("[") else { return nil }
        guard let data = trimmed.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data),
              let pretty = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]),
              let output = String(data: pretty, encoding: .utf8) else { return nil }
        return output
    }

    static func decodeBase64(_ text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count >= 8, trimmed.count % 4 == 0,
              let data = Data(base64Encoded: trimmed),
              let output = String(data: data, encoding: .utf8),
              !output.isEmpty else { return nil }
        return output
    }
}
