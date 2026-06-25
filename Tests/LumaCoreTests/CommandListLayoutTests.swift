import Foundation
import Testing
import LumaCore

@Test func commandRegistryHintForActivePrefix() {
    let registry = BuiltInCommandRegistry.make()
    let hint = registry.hint(for: "rec 三体")
    #expect(hint?.trigger == "rec")
    #expect(hint?.title == "Log Record")
    #expect(hint?.example == "rec 三体 book done 9 #sci-fi")
}

@Test func commandRegistryHintNilForPlainSearch() {
    let registry = BuiltInCommandRegistry.make()
    #expect(registry.hint(for: "chrome") == nil)
}

@Test func commandRegistrySectionTitleForMedia() {
    let registry = BuiltInCommandRegistry.make()
    let media = ModuleIdentifier(rawValue: "luma.media")
    #expect(registry.sectionTitle(for: media) == "RECORDS")
}

@Test func commandListLayoutTargetedUsesSingleSection() {
    let registry = BuiltInCommandRegistry.make()
    let media = ModuleIdentifier(rawValue: "luma.media")
    let item = ResultItem(
        id: ResultID(module: media, key: "a"),
        title: "A",
        titleAttributed: "A",
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: media, key: "a"),
            title: "Open",
            kind: .noop
        ),
        rankingHints: RankingHints()
    )
    let layout = CommandListLayout.build(
        items: [item],
        route: .targeted(module: media, trigger: "rec", payload: ""),
        registry: registry
    )
    guard case .sectioned(let sections) = layout else {
        Issue.record("Expected sectioned layout")
        return
    }
    #expect(sections.count == 1)
    #expect(sections[0].title == "RECORDS")
    #expect(sections[0].items.count == 1)
}

@Test func commandListLayoutGlobalSearchGroupsByModule() {
    let registry = BuiltInCommandRegistry.make()
    let apps = ModuleIdentifier(rawValue: "luma.apps")
    let projects = ModuleIdentifier(rawValue: "luma.projects")
    let appItem = ResultItem(
        id: ResultID(module: apps, key: "app"),
        title: "Cursor",
        titleAttributed: "Cursor",
        icon: .none,
        primaryAction: Action(id: ActionID(module: apps, key: "app"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let projectItem = ResultItem(
        id: ResultID(module: projects, key: "proj"),
        title: "Luma",
        titleAttributed: "Luma",
        icon: .none,
        primaryAction: Action(id: ActionID(module: projects, key: "proj"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let layout = CommandListLayout.build(
        items: [appItem, projectItem],
        route: .globalSearch("cursor"),
        registry: registry
    )
    guard case .sectioned(let sections) = layout else {
        Issue.record("Expected sectioned layout")
        return
    }
    #expect(sections.map(\.title) == ["APPS", "PROJECTS"])
}

@Test func listRowsRenderSectionedSearchResults() {
    let media = ModuleIdentifier(rawValue: "luma.media")
    let item = ResultItem(
        id: ResultID(module: media, key: "a"),
        title: "三体",
        titleAttributed: "三体",
        icon: .none,
        primaryAction: Action(id: ActionID(module: media, key: "a"), title: "Open", kind: .noop),
        rankingHints: RankingHints()
    )
    let rows = LauncherListRows.rows(
        for: [item],
        layout: .sectioned([ResultSection(title: "RECORDS", items: [item])])
    )
    #expect(rows.count == 2)
    if case .sectionHeader(let title, _) = rows[0].kind {
        #expect(title == "RECORDS")
    } else {
        Issue.record("Expected section header")
    }
}

@Test func queryCarriesParsedCommand() {
    let media = ModuleIdentifier(rawValue: "luma.media")
    let command = ParsedCommand(trigger: "rec", payload: "三体", module: media)
    let query = Query(raw: "rec 三体", sequence: 1, command: command)
    #expect(query.commandPayload == "三体")
    #expect(query.command?.trigger == "rec")
}
