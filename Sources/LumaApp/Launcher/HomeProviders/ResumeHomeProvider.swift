import Foundation
import LumaCore
import LumaModules

struct ResumeHomeProvider: LauncherHomeProvider {
  func items() async -> [ResultItem] {
    let state = LauncherResumeStore.load()
    var rows: [ResultItem] = []

    if !state.translateSource.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      let preview = String(state.translateSource.prefix(48))
      rows.append(ResultItem(
        id: ResultID(module: .translate, key: "resume.translate"),
        title: "Resume translation",
        titleAttributed: AttributedString("Resume translation"),
        subtitle: preview,
        icon: .symbol("character.bubble"),
        primaryAction: Action(
          id: ActionID(module: .translate, key: "resume"),
          title: "Open Translate",
          kind: .openModuleDetail(.translate, payload: nil)
        ),
        rankingHints: RankingHints(basePriority: 95),
        rowKind: .starter
      ))
    }

    if let data = state.snippetDraftJSON,
       let draft = try? JSONDecoder().decode(SnippetDraft.self, from: data) {
      let payload = (try? ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))) ?? Data()
      rows.append(ResultItem(
        id: ResultID(module: .snippets, key: "resume.snippet"),
        title: "Resume snippet draft",
        titleAttributed: AttributedString("Resume snippet draft"),
        subtitle: draft.trigger,
        icon: .symbol("text.badge.plus"),
        primaryAction: Action(
          id: ActionID(module: .snippets, key: "resume"),
          title: "Open Snippets",
          kind: .openModuleDetail(.snippets, payload: payload)
        ),
        rankingHints: RankingHints(basePriority: 94),
        rowKind: .starter
      ))
    }

    if let data = state.quicklinkDraftJSON,
       let draft = try? JSONDecoder().decode(URLQuicklinkDraft.self, from: data) {
      let payload = (try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))) ?? Data()
      rows.append(ResultItem(
        id: ResultID(module: .quicklinks, key: "resume.quicklink"),
        title: "Resume quicklink draft",
        titleAttributed: AttributedString("Resume quicklink draft"),
        subtitle: draft.trigger,
        icon: .symbol("link"),
        primaryAction: Action(
          id: ActionID(module: .quicklinks, key: "resume"),
          title: "Open Quicklinks",
          kind: .openModuleDetail(.quicklinks, payload: payload)
        ),
        rankingHints: RankingHints(basePriority: 93),
        rowKind: .starter
      ))
    }

    if let todoText = state.todoCaptureText,
       !todoText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      rows.append(ResultItem(
        id: ResultID(module: .todo, key: "resume.todo"),
        title: "Resume todo capture",
        titleAttributed: AttributedString("Resume todo capture"),
        subtitle: String(todoText.prefix(48)),
        icon: .symbol("checklist"),
        primaryAction: Action(
          id: ActionID(module: .todo, key: "resume"),
          title: "Open Todo",
          kind: .replaceQuery(TodoModule.resumeQuery(forCapture: todoText))
        ),
        rankingHints: RankingHints(basePriority: 92),
        rowKind: .starter
      ))
    }

    if let moduleRaw = state.moduleRaw,
       moduleRaw != ModuleIdentifier(rawValue: "luma.translate").rawValue,
       state.snippetDraftJSON == nil,
       state.quicklinkDraftJSON == nil {
      let moduleID = ModuleIdentifier(rawValue: moduleRaw)
      let trimmed = state.query.trimmingCharacters(in: .whitespacesAndNewlines)
      let resolved = LauncherModuleResumeQuery.normalizedQuery(for: moduleID, raw: state.query)
      let display = trimmed.isEmpty ? resolved.trimmingCharacters(in: .whitespacesAndNewlines) : trimmed
      if !display.isEmpty {
        rows.append(ResultItem(
          id: ResultID(module: moduleID, key: "resume.module"),
          title: LauncherModuleResumeQuery.resumeTitle(for: moduleID),
          titleAttributed: AttributedString(LauncherModuleResumeQuery.resumeTitle(for: moduleID)),
          subtitle: display,
          icon: .symbol("arrow.uturn.backward"),
          primaryAction: Action(
            id: ActionID(module: moduleID, key: "resume"),
            title: "Restore query",
            kind: .replaceQuery(resolved)
          ),
          rankingHints: RankingHints(basePriority: 91),
          rowKind: .starter
        ))
      }
    }

    return Array(rows.prefix(3))
  }
}
