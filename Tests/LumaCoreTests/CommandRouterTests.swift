import Foundation
import Testing
import LumaCore

@Test func commandRouterEmptyQueryRoutesToEmpty() {
    let router = CommandRouter()
    #expect(router.route(raw: "") == .empty)
    #expect(router.route(raw: "   ") == .empty)
}

@Test func commandRouterQuestionMarkRoutesGlobalHelp() {
    let router = CommandRouter()
    #expect(router.route(raw: "?") == .help(module: nil))
    #expect(router.route(raw: "help") == .help(module: nil))
}

@Test func commandRouterModuleHelpRoutesMedia() {
    let router = CommandRouter()
    let media = ModuleIdentifier(rawValue: "luma.media")
    #expect(router.route(raw: "rec ?") == .help(module: media))
    #expect(router.route(raw: "rec help") == .help(module: media))
}

@Test func commandRouterRecRoutesTargetedMedia() {
    let router = CommandRouter()
    let media = ModuleIdentifier(rawValue: "luma.media")
    #expect(router.route(raw: "rec 三体") == .targeted(module: media, trigger: "rec", payload: "三体"))
    #expect(router.route(raw: "rec") == .targeted(module: media, trigger: "rec", payload: ""))
}

@Test func commandRouterProjectsRoutesTargeted() {
    let router = CommandRouter()
    let projects = ModuleIdentifier(rawValue: "luma.projects")
    #expect(router.route(raw: "p luma") == .targeted(module: projects, trigger: "p", payload: "luma"))
}

@Test func commandRouterWindowLayoutsRoutesTargeted() {
    let router = CommandRouter()
    let layouts = ModuleIdentifier(rawValue: "luma.window-layouts")
    #expect(router.route(raw: "win left") == .targeted(module: layouts, trigger: "win", payload: "left"))
}

@Test func commandRouterBareTranslateRoutesTargeted() {
    let router = CommandRouter()
    let translate = ModuleIdentifier(rawValue: "luma.translate")
    #expect(router.route(raw: "tr") == .targeted(module: translate, trigger: "tr", payload: ""))
}

@Test func commandRouterTypoPrefixSuggestsWin() {
    let router = CommandRouter()
    let route = router.route(raw: "wni left")
    guard case .unknownPrefix(let prefix, let remainder, let suggestions) = route else {
        Issue.record("Expected unknownPrefix route")
        return
    }
    #expect(prefix == "wni")
    #expect(remainder == "left")
    #expect(suggestions.first?.trigger == "win")
}

@Test func commandRouterPlainSearchRoutesGlobal() {
    let router = CommandRouter()
    guard case .globalSearch(let query) = router.route(raw: "chrome") else {
        Issue.record("Expected globalSearch route")
        return
    }
    #expect(query == "chrome")
}

@Test func commandRouterPlainShortSearchDoesNotBecomeCommandTypo() {
    let router = CommandRouter()
    guard case .globalSearch(let query) = router.route(raw: "git status") else {
        Issue.record("Expected globalSearch route")
        return
    }
    #expect(query == "git status")
}

@Test func commandRouterNearTriggerTypoStillSuggestsCommand() {
    let router = CommandRouter()
    let route = router.route(raw: "re dune")
    guard case .unknownPrefix(let prefix, let remainder, let suggestions) = route else {
        Issue.record("Expected unknownPrefix route")
        return
    }
    #expect(prefix == "re")
    #expect(remainder == "dune")
    #expect(suggestions.first?.trigger == "rec")
}

@Test func commandRouterAppChromeRoutesGlobalSearch() {
    let router = CommandRouter()
    guard case .globalSearch(let query) = router.route(raw: "app chrome") else {
        Issue.record("Expected globalSearch for app chrome")
        return
    }
    #expect(query == "app chrome")
}

@Test func commandRouterAppTopRoutesTargetedApps() {
    let router = CommandRouter()
    let apps = ModuleIdentifier(rawValue: "luma.apps")
    #expect(router.route(raw: "app top") == .targeted(module: apps, trigger: "app", payload: "top"))
}

@Test func commandRouterBareERoutesGlobalSearch() {
    let router = CommandRouter()
    guard case .globalSearch(let query) = router.route(raw: "e") else {
        Issue.record("Expected globalSearch route for bare e")
        return
    }
    #expect(query == "e")
    guard case .globalSearch(let eventQuery) = router.route(raw: "event meet john") else {
        Issue.record("Expected globalSearch route for event prefix")
        return
    }
    #expect(eventQuery == "event meet john")
}

@Test func globalHelpIncludesFooterAndHelpPreview() {
    let rows = CommandEntryResults.globalHelp(registry: BuiltInCommandRegistry.make())
    #expect(rows.last?.id.key == "help.footer")
    #expect(rows.last?.subtitle == "Example: rec ?")
    let recRow = rows.first { $0.id.key == "help.rec" }
    #expect(recRow?.subtitle == "Log or search books, movies, shows, anime, and games")
}

@Test func globalHelpSortsByDiscoverPriority() {
    let rows = CommandEntryResults.globalHelp(registry: BuiltInCommandRegistry.make())
    let commandRows = rows.filter { $0.id.key.hasPrefix("help.") && $0.id.key != "help.footer" }
    #expect(commandRows.first?.id.key == "help.p")
    #expect(commandRows.contains { $0.id.key == "help.rec" })
}
