import Foundation
import LumaCore

/// Single-pass routing and hint state for one keystroke.
@MainActor
struct NormalizedQueryState: Equatable {
    enum ResultsRouteKind: Equatable {
        case empty
        case workbench(WorkbenchCommandRoute)
        case command(CommandRoute)
    }

    let raw: String
    let trimmed: String
    let workbenchRoute: WorkbenchCommandRoute
    let commandRoute: CommandRoute
    let hint: CommandHint?
    let helpTrigger: String?
    let resultsRouteKind: ResultsRouteKind

    init(raw: String, viewModel: LauncherViewModel) {
        self.raw = raw
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        self.trimmed = trimmed
        let workbenchRoute = viewModel.workbenchRoute(for: raw)
        self.workbenchRoute = workbenchRoute
        if workbenchRoute != .none {
            self.commandRoute = .empty
            self.hint = viewModel.workbenchCommandHint(for: workbenchRoute, raw: raw)
            self.resultsRouteKind = .workbench(workbenchRoute)
        } else {
            let route = viewModel.commandRouter.route(raw: raw)
            self.commandRoute = route
            self.hint = viewModel.commandRouter.registry.hint(for: raw)
            self.resultsRouteKind = trimmed.isEmpty ? .empty : .command(route)
        }
        self.helpTrigger = trimmed.split(separator: " ", maxSplits: 1).first.map(String.init)
    }
}
