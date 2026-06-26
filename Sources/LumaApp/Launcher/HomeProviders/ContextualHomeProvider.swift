import AppKit
import ApplicationServices
import Foundation
import LumaCore
import LumaModules
import LumaServices

actor ContextualSelectionCache {
    static let shared = ContextualSelectionCache()

    private var cachedText: String?
    private var cachedAt: Date?
    private var refreshInFlight = false
    private let ttl: TimeInterval = 1.5

    func snapshot() -> String? {
        if let cachedAt, Date().timeIntervalSince(cachedAt) < ttl {
            return cachedText
        }
        return nil
    }

    func refreshIfNeeded() {
        if let cachedAt, Date().timeIntervalSince(cachedAt) < ttl { return }
        if refreshInFlight { return }
        refreshInFlight = true
        Task { [weak self] in
            let text = await MainActor.run { Self.readSelectedText() }
            await self?.store(text)
        }
    }

    private func store(_ text: String?) {
        cachedText = text
        cachedAt = Date()
        refreshInFlight = false
    }

    @MainActor
    private static func readSelectedText() -> String? {
        guard AXService.isProcessTrusted(),
              let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused else { return nil }
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element as! AXUIElement, kAXSelectedTextAttribute as CFString, &value) == .success,
              let text = value as? String else { return nil }
        return text
    }
}

struct ContextualHomeProvider: LauncherHomeProvider {
    private let todoModule: TodoModule?
    private let mediaModule: MediaModule?

    init(todoModule: TodoModule? = nil, mediaModule: MediaModule? = nil) {
        self.todoModule = todoModule
        self.mediaModule = mediaModule
    }

    func items() async -> [ResultItem] {
        var suggestions: [ResultItem] = []
        await ContextualSelectionCache.shared.refreshIfNeeded()

        if let text = await ContextualSelectionCache.shared.snapshot(),
           !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
           TranslationUserMessages.shouldTranslate(text) {
            suggestions.append(translateSelectedRow(text: text))
        }

        if suggestions.count < 3,
           let clipRow = await clipboardRow() {
            suggestions.append(clipRow)
        }

        if suggestions.count < 3,
           let recordsRow = await continueRecordsRow() {
            suggestions.append(recordsRow)
        }

        if suggestions.count < 3,
           let todoRow = await todayTodosRow() {
            suggestions.append(todoRow)
        }

        return Array(suggestions.prefix(3))
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
        guard let value = await ClipboardPasteboardCache.shared.snapshot(),
              !value.isEmpty else { return nil }
        let preview = String(value.prefix(48))
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
