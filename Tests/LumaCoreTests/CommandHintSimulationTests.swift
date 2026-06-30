import Foundation
import Testing
import LumaCore
import LumaModules

/// Simulates launcher input-box flows: routing, hint bar, placeholder, help.
private struct InputSimulation {
    let input: String
    let expectHint: Bool
    let expectRouteKind: RouteKind
    let label: String

    enum RouteKind: Equatable {
        case empty
        case globalSearch
        case targeted
        case helpGlobal
        case helpModule
        case unknownPrefix
    }
}

@Test func simulateLauncherInputBoxFlows() {
    let registry = BuiltInCommandRegistry.make()
    let router = CommandRouter(registry: registry)

    let scenarios: [InputSimulation] = [
        // Home / empty
        InputSimulation(input: "", expectHint: false, expectRouteKind: .empty, label: "empty home"),
        InputSimulation(input: "   ", expectHint: false, expectRouteKind: .empty, label: "whitespace"),

        // Global search — no hint bar
        InputSimulation(input: "chrome", expectHint: false, expectRouteKind: .globalSearch, label: "plain search"),
        InputSimulation(input: "git status", expectHint: false, expectRouteKind: .globalSearch, label: "multi-word search"),
        InputSimulation(input: "e", expectHint: false, expectRouteKind: .globalSearch, label: "single-char non-command"),

        // Command prefixes — hint bar visible
        InputSimulation(input: "word", expectHint: true, expectRouteKind: .targeted, label: "word bare"),
        InputSimulation(input: "word ", expectHint: true, expectRouteKind: .targeted, label: "word trailing space"),
        InputSimulation(input: "word review", expectHint: true, expectRouteKind: .targeted, label: "word with payload"),
        InputSimulation(input: "wb", expectHint: true, expectRouteKind: .targeted, label: "word alias"),
        InputSimulation(input: "clip", expectHint: true, expectRouteKind: .targeted, label: "clip bare"),
        InputSimulation(input: "clip jwt", expectHint: true, expectRouteKind: .targeted, label: "clip search"),
        InputSimulation(input: "n", expectHint: true, expectRouteKind: .targeted, label: "notes bare"),
        InputSimulation(input: "n daily", expectHint: true, expectRouteKind: .targeted, label: "notes daily"),
        InputSimulation(input: "tr", expectHint: true, expectRouteKind: .targeted, label: "translate bare"),
        InputSimulation(input: "tr hello", expectHint: true, expectRouteKind: .targeted, label: "translate text"),
        InputSimulation(input: "rec", expectHint: true, expectRouteKind: .targeted, label: "records bare"),
        InputSimulation(input: "p luma", expectHint: true, expectRouteKind: .targeted, label: "project open"),
        InputSimulation(input: "win left", expectHint: true, expectRouteKind: .targeted, label: "window layout"),
        InputSimulation(input: "t buy milk", expectHint: true, expectRouteKind: .targeted, label: "todo create"),
        InputSimulation(input: "s git", expectHint: true, expectRouteKind: .targeted, label: "snippet search"),
        InputSimulation(input: "sec aws", expectHint: true, expectRouteKind: .targeted, label: "secrets search"),
        InputSimulation(input: "app top", expectHint: true, expectRouteKind: .targeted, label: "apps memory"),
        InputSimulation(input: "settings", expectHint: true, expectRouteKind: .targeted, label: "settings"),
        InputSimulation(input: "quit", expectHint: true, expectRouteKind: .targeted, label: "quit"),

        // app + name → global search (special case), but hint still shows app
        InputSimulation(input: "app chrome", expectHint: true, expectRouteKind: .globalSearch, label: "app name global"),

        // Help flows
        InputSimulation(input: "?", expectHint: false, expectRouteKind: .helpGlobal, label: "global help"),
        InputSimulation(input: "help", expectHint: false, expectRouteKind: .helpGlobal, label: "help keyword"),
        InputSimulation(input: "word ?", expectHint: true, expectRouteKind: .helpModule, label: "module help suffix"),
        InputSimulation(input: "? word", expectHint: true, expectRouteKind: .helpModule, label: "help prefix form"),
        InputSimulation(input: "help clip", expectHint: true, expectRouteKind: .helpModule, label: "help clip"),

        // Typo suggestions
        InputSimulation(input: "wni left", expectHint: false, expectRouteKind: .unknownPrefix, label: "typo win"),
    ]

    for scenario in scenarios {
        let hint = registry.hint(for: scenario.input)
        let hasHint = hint != nil
        #expect(hasHint == scenario.expectHint, "\(scenario.label): hint visibility for \"\(scenario.input)\"")

        let routeKind = classifyRoute(router.route(raw: scenario.input))
        #expect(routeKind == scenario.expectRouteKind, "\(scenario.label): route for \"\(scenario.input)\" got \(routeKind)")

        if hasHint, let hint {
            #expect(!hint.usageFormat.isEmpty, "\(scenario.label): missing format")
            #expect(!hint.description.isEmpty, "\(scenario.label): missing description")
        }
    }
}

@Test func simulateAllBuiltInTriggersShowHint() {
    let registry = BuiltInCommandRegistry.make()
    for command in registry.allCommands {
        for trigger in command.allTriggers {
            let hint = registry.hint(for: trigger)
            #expect(hint != nil, "No hint for trigger \"\(trigger)\" (\(command.id))")
            #expect(hint?.usageFormat.contains(trigger) == true || hint?.usageFormat.contains(command.primaryTrigger) == true,
                    "Format should reference trigger for \(trigger)")
        }
    }
}

@Test func simulateKeyboardBehaviorsUnchanged() {
    // Return → run first item is handled in LauncherRootController.activateReturn (not key router)
    // Tab / ⌘K → open action panel when results exist
    #expect(LauncherKeyRouter.route(command: .tab, mode: .results, itemCount: 3, actionPanelVisible: false) == .openActionPanel)
    #expect(LauncherKeyRouter.route(command: .actionPanel, mode: .results, itemCount: 3, actionPanelVisible: false) == .openActionPanel)
    #expect(LauncherKeyRouter.route(command: .tab, mode: .home, itemCount: 0, actionPanelVisible: false) == .handled)
    #expect(LauncherKeyRouter.route(command: .down, mode: .results, itemCount: 5, actionPanelVisible: false) == .moveSelection(delta: 1))
    #expect(LauncherKeyRouter.route(command: .commandNumber(1), mode: .results, itemCount: 5, actionPanelVisible: false) == .jumpToFlatIndex(0))
    // Esc is handled by searchBar.onEscape → handleEscape, not LauncherKeyRouter
}

@Test func simulatePlaceholderTracksCommand() {
    let registry = BuiltInCommandRegistry.make()
    #expect(registry.placeholder(for: "word review").contains("vocabulary") || registry.placeholder(for: "word").contains("vocabulary"))
    #expect(registry.placeholder(for: "chrome") == CommandRegistry.defaultPlaceholder)
}

private func classifyRoute(_ route: CommandRoute) -> InputSimulation.RouteKind {
    switch route {
    case .empty: return .empty
    case .globalSearch: return .globalSearch
    case .targeted: return .targeted
    case .help(nil): return .helpGlobal
    case .help(.some): return .helpModule
    case .unknownPrefix: return .unknownPrefix
    case .suggestion: return .unknownPrefix
    }
}
