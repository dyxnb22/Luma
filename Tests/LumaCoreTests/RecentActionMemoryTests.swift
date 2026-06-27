import Foundation
import Testing
import LumaCore

@Test func recentActionMemoryRecordsAndReplays() async {
  let url = FileManager.default.temporaryDirectory.appendingPathComponent("recent-\(UUID().uuidString).json")
  defer { try? FileManager.default.removeItem(at: url) }
  let memory = RecentActionMemory(maxEntries: 4, persistenceURL: url)
  let module = ModuleIdentifier(rawValue: "luma.notes")
  let item = ResultItem(
    id: ResultID(module: module, key: "capture"),
    title: "Append to note",
    titleAttributed: AttributedString("Append to note"),
    icon: .symbol("note"),
    primaryAction: Action(
      id: ActionID(module: module, key: "capture"),
      title: "Append",
      kind: .copyToPasteboard("hello")
    ),
    rankingHints: RankingHints()
  )
  let action = item.primaryAction
  await memory.record(action: action, item: item)
  let recent = await memory.recent(limit: 2)
  #expect(recent.count == 1)
  #expect(recent[0].title == "Append to note")
}

@Test func recentActionMemorySkipsLaunchApp() async {
  let url = FileManager.default.temporaryDirectory.appendingPathComponent("recent-\(UUID().uuidString).json")
  defer { try? FileManager.default.removeItem(at: url) }
  let memory = RecentActionMemory(maxEntries: 4, persistenceURL: url)
  let module = ModuleIdentifier(rawValue: "luma.apps")
  let item = ResultItem(
    id: ResultID(module: module, key: "safari"),
    title: "Safari",
    titleAttributed: AttributedString("Safari"),
    icon: .bundleID("com.apple.Safari"),
    primaryAction: Action(
      id: ActionID(module: module, key: "launch"),
      title: "Open",
      kind: .launchApp(URL(fileURLWithPath: "/Applications/Safari.app"))
    ),
    rankingHints: RankingHints()
  )
  await memory.record(action: item.primaryAction, item: item)
  #expect(await memory.recent().isEmpty)
}

@Test func permissionResultBuilderUsesRequestAndSettingsActions() {
  let module = ModuleIdentifier(rawValue: "luma.todo")
  let row = PermissionResultBuilder.row(
    spec: PermissionCardSpec(
      module: module,
      title: "Access needed",
      explanation: "Explain why",
      icon: .symbol("lock"),
      requestAction: Action(
        id: ActionID(module: module, key: "request"),
        title: "Allow",
        kind: .noop
      ),
      settingsAction: Action(
        id: ActionID(module: module, key: "settings"),
        title: "Open System Settings",
        kind: .noop
      ),
      accessDenied: false
    )
  )
  #expect(row.primaryAction.title == "Allow")
  #expect(row.secondaryActions.first?.title == "Open System Settings")
}
