import AppKit
import Foundation
import LumaCore
import LumaModules
import LumaServices

struct ContextualHomeProvider: LauncherHomeProvider, ContextualHomeSectionProvider {
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
    let sections = await rankedSectionItems()
    return Array((sections.continue + sections.create).prefix(4))
  }

  func sectionedItems() async -> (continue: [ResultItem], create: [ResultItem]) {
    await rankedSectionItems()
  }

  private func rankedSectionItems() async -> (continue: [ResultItem], create: [ResultItem]) {
    let memory = suggestionMemory

    // Run all independent fetches concurrently so their latencies overlap.
    async let projectContext = CurrentProjectService.shared.snapshot()
    async let selectedText = SelectionSnapshotService.shared.snapshot()
    async let dailyRow = continueDailyNoteRow()
    async let todoRow = topTodoRow()
    async let transformRow = clipboardTransformRow()
    async let noteRow = saveClipboardToNoteRow()
    async let snippetRow = saveClipboardAsSnippetRow()
    async let quicklinkRow = saveURLAsQuicklinkRow()
    async let recordsRow = continueRecordsRow()

    let (context, text, daily, todo, transform, note, snippet, quicklink, records) = await (
      projectContext, selectedText, dailyRow, todoRow, transformRow, noteRow, snippetRow, quicklinkRow, recordsRow
    )

    var candidates: [(item: ResultItem, key: String, kind: HomeSuggestionKind, base: Int)] = []

    if let context {
      await memory.boostSessionContext(key: "contextual.current")
      if let row = currentProjectRow(context: context) {
        candidates.append((row, "contextual.current", .continueFlow, 88))
      }
    }

    if let text,
       !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
       TranslationUserMessages.shouldTranslate(text) {
      candidates.append((translateSelectedRow(text: text), "contextual.translate", .utility, 90))
    }

    if let daily { candidates.append((daily, "contextual.daily", .continueFlow, 87)) }
    if let todo { candidates.append((todo, "contextual.todo", .continueFlow, 86)) }
    if let transform { candidates.append((transform, transform.id.key, .transform, 84)) }
    if let note { candidates.append((note, "contextual.clip-note", .create, 83)) }
    if let snippet { candidates.append((snippet, "contextual.clip-snippet", .create, 82)) }
    if let quicklink { candidates.append((quicklink, "contextual.url-quicklink", .create, 81)) }
    if let records { candidates.append((records, "contextual.records", .continueFlow, 75)) }

    var ranked: [(item: ResultItem, kind: HomeSuggestionKind, priority: Int)] = []
    for candidate in candidates {
      guard await memory.isEligible(key: candidate.key, kind: candidate.kind) else { continue }
      let adjusted = await memory.adjustedPriority(base: candidate.base, key: candidate.key, kind: candidate.kind)
      ranked.append((candidate.item, candidate.kind, adjusted))
    }

    ranked.sort { $0.priority > $1.priority }
    let continueFlow = ranked
      .filter { $0.kind == .continueFlow }
      .prefix(4)
      .map(\.item)
    let create = ranked
      .filter { $0.kind != .continueFlow }
      .prefix(4)
      .map(\.item)
    return (continue: Array(continueFlow), create: Array(create))
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

  private func clipboardTransformRow() async -> ResultItem? {
    guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
          !preview.isEmpty else { return nil }

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

  private func saveClipboardToNoteRow() async -> ResultItem? {
    guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
          !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
          ClipboardTextOps.classify(preview) != .url else { return nil }
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

  private func saveClipboardAsSnippetRow() async -> ResultItem? {
    guard let preview = await ClipboardPasteboardCache.shared.snapshot(),
          !preview.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
          preview.count >= 8,
          ClipboardTextOps.classify(preview) != .url else { return nil }
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
