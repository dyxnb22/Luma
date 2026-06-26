import AppKit
import Foundation
import LumaCore
import LumaModules
import LumaServices

struct ContextualHomeProvider: LauncherHomeProvider {
    private let todoModule: TodoModule?
    private let mediaModule: MediaModule?

    init(todoModule: TodoModule? = nil, mediaModule: MediaModule? = nil) {
        self.todoModule = todoModule
        self.mediaModule = mediaModule
    }

    func items() async -> [ResultItem] {
        var suggestions: [ResultItem] = []

        if let text = await SelectionSnapshotService.shared.snapshot(),
           !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
           TranslationUserMessages.shouldTranslate(text) {
            suggestions.append(translateSelectedRow(text: text))
        }

        if suggestions.count < 4,
           let projectRow = await currentProjectRow() {
            suggestions.append(projectRow)
        }

        if suggestions.count < 4,
           let transformRow = await clipboardTransformRow() {
            suggestions.append(transformRow)
        }

        if suggestions.count < 4,
           let clipRow = await clipboardRow() {
            suggestions.append(clipRow)
        }

        if suggestions.count < 4,
           let recordsRow = await continueRecordsRow() {
            suggestions.append(recordsRow)
        }

        if suggestions.count < 4,
           let todoRow = await todayTodosRow() {
            suggestions.append(todoRow)
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
            rankingHints: RankingHints(basePriority: 85),
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
            rankingHints: RankingHints(basePriority: 82),
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

    private func clipboardRow() async -> ResultItem? {
        guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
              !preview.isEmpty else { return nil }
        return ResultItem(
            id: ResultID(module: .clipboard, key: "contextual.clipboard"),
            title: "Open last clipboard item",
            titleAttributed: AttributedString("Open last clipboard item"),
            subtitle: preview,
            icon: .symbol("doc.on.clipboard"),
            primaryAction: Action(
                id: ActionID(module: .clipboard, key: "contextual.open"),
                title: "Open Clipboard",
                kind: .openModuleDetail(.clipboard, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: 80),
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

    private func todayTodosRow() async -> ResultItem? {
        guard let todoModule else { return nil }
        let count: Int
        do {
            count = try await todoModule.todayDueCount()
        } catch {
            return nil
        }
        guard count > 0 else { return nil }
        return ResultItem(
            id: ResultID(module: .todo, key: "contextual.today"),
            title: "Open today's todos",
            titleAttributed: AttributedString("Open today's todos"),
            subtitle: "\(count) due today",
            icon: .symbol("checkmark.circle"),
            primaryAction: Action(
                id: ActionID(module: .todo, key: "contextual.open"),
                title: "Open Todo",
                kind: .openModuleDetail(.todo, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: 70),
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
