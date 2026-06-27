import Foundation
import Testing
@testable import LumaCore

@Test func rankerDropsNonMatchingQuery() {
    let module = ModuleIdentifier(rawValue: "test")
    let id = ResultID(module: module, key: "safari")
    let action = Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop)
    let item = ResultItem(
        id: id,
        title: "Safari",
        titleAttributed: AttributedString("Safari"),
        icon: .none,
        primaryAction: action,
        rankingHints: RankingHints()
    )
    let query = Query(raw: "zzz", sequence: 1)
    #expect(Ranker.score(item: item, query: query, usage: nil) == -.infinity)
}

@Test func rankerUsesCommandPayloadForTargetedQueries() {
    let module = ModuleIdentifier(rawValue: "luma.killProcess")
    let item = ResultItem(
        id: ResultID(module: module, key: "preview"),
        title: "Preview",
        titleAttributed: AttributedString("Preview"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop),
        rankingHints: RankingHints()
    )
    let query = Query(
        raw: "kill preview",
        sequence: 1,
        command: ParsedCommand(trigger: "kill", payload: "preview", module: module)
    )
    #expect(Ranker.score(item: item, query: query, usage: nil) > -.infinity)
}

@Test func rankerMatchesKillProcessSubtitleBundleID() {
    let module = ModuleIdentifier(rawValue: "luma.killProcess")
    let item = ResultItem(
        id: ResultID(module: module, key: "preview"),
        title: "预览",
        titleAttributed: AttributedString("预览"),
        subtitle: "com.apple.Preview · 128 MB",
        icon: .none,
        primaryAction: Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop),
        rankingHints: RankingHints()
    )
    let query = Query(
        raw: "kill preview",
        sequence: 1,
        command: ParsedCommand(trigger: "kill", payload: "preview", module: module)
    )
    #expect(Ranker.score(item: item, query: query, usage: nil) > -.infinity)
}

@Test func rankerKeepsModuleHomeRowsForEmptyCommandPayload() {
    let module = ModuleIdentifier(rawValue: "luma.projects")
    let item = ResultItem(
        id: ResultID(module: module, key: "luma"),
        title: "Luma",
        titleAttributed: AttributedString("Luma"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop),
        rankingHints: RankingHints()
    )
    let query = Query(
        raw: "proj",
        sequence: 1,
        command: ParsedCommand(trigger: "proj", payload: "", module: module)
    )
    #expect(Ranker.score(item: item, query: query, usage: nil) > -.infinity)
}

@Test func rankerMatchesQuicklinkTriggerTokenInMultiWordQuery() {
    let module = ModuleIdentifier(rawValue: "luma.quicklinks")
    let item = ResultItem(
        id: ResultID(module: module, key: "gh"),
        title: "GitHub",
        titleAttributed: AttributedString("GitHub"),
        icon: .none,
        primaryAction: Action(id: ActionID(module: module, key: "noop"), title: "Noop", kind: .noop),
        rankingHints: RankingHints()
    )
    let query = Query(raw: "gh swift package", sequence: 1)
    #expect(Ranker.score(item: item, query: query, usage: nil) > -.infinity)
}
