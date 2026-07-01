import Foundation
import Testing
import LumaCore
import LumaModules

@Test func appsBarePayloadShadowsToGlobalSearch() {
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    guard case .globalSearch(let query) = router.route(raw: "app chrome") else {
        Issue.record("Expected globalSearch for app chrome")
        return
    }
    #expect(query == "app chrome")
}

@Test func appsTopStaysTargeted() {
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    let apps = ModuleIdentifier(rawValue: "luma.apps")
    #expect(router.route(raw: "app top") == .targeted(module: apps, trigger: "app", payload: "top"))
}

@Test func translateBareUsesOpenDetailBehavior() {
    let registry = BuiltInCommandRegistry.make()
    let command = registry.command(forTrigger: "tr")
    #expect(command?.bareBehavior == .openDetail)
}

@Test func notesBareUsesOpenDetailBehavior() {
    let registry = BuiltInCommandRegistry.make()
    let notes = registry.command(forTrigger: "notes")
    #expect(notes?.bareBehavior == .openDetail)
}

@Test func resolveRunDetectsToggleWindowsRow() {
    let item = OpenAppsResultBuilder.expandableResultItem(
        for: RunningAppSnapshot(
            bundleID: "com.example.app",
            name: "Example",
            appPath: "/Applications/Example.app",
            windowCount: 2
        ),
        isExpanded: false,
        secondaryActions: []
    )
    guard case .toggleOpenAppWindows(let bundleID) = LauncherKeyRouter.resolveRun(item: item) else {
        Issue.record("Expected toggleOpenAppWindows")
        return
    }
    #expect(bundleID == "com.example.app")
}

@Test func actionReturnHintForOpenDetail() {
    let wordbook = ModuleIdentifier(rawValue: "luma.wordbook")
    let action = Action(
        id: ActionID(module: wordbook, key: "open"),
        title: "Open",
        kind: .openModuleDetail(wordbook, payload: nil)
    )
    #expect(action.returnHint == "Open")
    #expect(action.keepsLauncherVisible)
}

@Test func replaceQueryReturnHint() {
    let action = Action(
        id: ActionID(module: .commandEntry, key: "replace"),
        title: "Use apps",
        kind: .replaceQuery("apps ")
    )
    #expect(action.returnHint == "Replace query")
}

@Test func informationalRowReturnHint() {
    let item = ResultItem(
        id: ResultID(module: ModuleIdentifier(rawValue: "luma.notes"), key: "doctor"),
        title: "Stats",
        titleAttributed: "Stats",
        icon: .none,
        primaryAction: Action(
            id: ActionID(module: ModuleIdentifier(rawValue: "luma.notes"), key: "noop"),
            title: "OK",
            kind: .noop
        ),
        rankingHints: RankingHints(),
        rowKind: .informational
    )
    #expect(item.returnHint == "Information only")
}

// MARK: - Bare-command Return dispatch invariants
// `performBareCommandAction` lives in LauncherRootController (AppKit). These
// tests pin the pure-Core invariants it depends on so changes there don't
// silently break the bare-command Return semantics documented in
// docs/specs/UX_BEHAVIOR_RULES.md.

@Test func bareOpenDetailCommandsTargetTheirModule() {
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    let registry = router.registry
    let cases: [(trigger: String, moduleRaw: String)] = [
        ("word", "luma.wordbook"),
        ("notes", "luma.notes"),
        ("clip",  "luma.clipboard"),
        ("tr",   "luma.translate"),
        ("rec",  "luma.media"),
        ("sec",  "luma.secrets"),
        ("t", "luma.todo"),
        ("todo", "luma.todo")
    ]
    for c in cases {
        guard case .targeted(let module, let trigger, let payload) = router.route(raw: c.trigger) else {
            Issue.record("Expected .targeted for bare `\(c.trigger)`")
            continue
        }
        #expect(module.rawValue == c.moduleRaw)
        #expect(trigger == registry.command(forModule: module)?.primaryTrigger)
        #expect(payload.isEmpty)
        #expect(registry.command(forModule: module)?.bareBehavior == .openDetail)
    }
}

@Test func bareOpenDetailReturnPrefersPanelOverRowResults() {
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    #expect(router.isBareOpenDetailReturn(raw: "notes"))
    #expect(router.isBareOpenDetailReturn(raw: "n"))
    #expect(router.isBareOpenDetailReturn(raw: "clip"))
    #expect(router.isBareOpenDetailReturn(raw: "word"))
    #expect(router.isBareOpenDetailReturn(raw: "word review"))
    #expect(!router.isBareOpenDetailReturn(raw: "notes daily"))
    #expect(!router.isBareOpenDetailReturn(raw: "n new idea"))
    #expect(!router.isBareOpenDetailReturn(raw: "word abandon"))
    #expect(router.isBareOpenDetailReturn(raw: "t"))
    #expect(router.isBareOpenDetailReturn(raw: "todo"))
    #expect(!router.isBareOpenDetailReturn(raw: "s"))
}

@Test func wordReviewRouteIsTargetedWithReviewPayload() {
    // The launcher reads this exact payload string to decide whether to set
    // `LauncherSharedState.pendingWordbookAutoStartReview` before opening detail.
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    guard case .targeted(let module, _, let payload) = router.route(raw: "word review") else {
        Issue.record("Expected .targeted for `word review`")
        return
    }
    #expect(module.rawValue == "luma.wordbook")
    #expect(payload.caseInsensitiveCompare("review") == .orderedSame)
}

@Test func snippetsBrowseDoesNotOpenDetailOnEmptyReturn() {
    let registry = BuiltInCommandRegistry.make()
    let snippets = registry.command(forTrigger: "s")
    #expect(snippets?.bareBehavior == .browse)
}

@Test func appsShadowReservesPayloadsKeepTargeted() {
    let router = CommandRouter(registry: BuiltInCommandRegistry.make())
    let registry = router.registry
    let apps = registry.command(forTrigger: "app")
    #expect(apps?.bareBehavior == .globalSearchShadow)
    #expect(apps?.bareReservedPayloads.contains("top") == true)
    guard case .targeted = router.route(raw: "app top") else {
        Issue.record("Expected `app top` to stay targeted")
        return
    }
}
