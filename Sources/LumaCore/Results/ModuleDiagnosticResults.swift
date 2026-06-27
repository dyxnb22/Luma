import Foundation

public enum ModuleDiagnosticResults {
    public static func informationalRow(module: ModuleIdentifier, diagnostic: ModuleDiagnostic) -> ResultItem {
        let icon: IconRef
        switch diagnostic.kind {
        case .timeout:
            icon = .symbol("clock.badge.exclamationmark")
        case .error:
            icon = .symbol("exclamationmark.triangle")
        case .degraded:
            icon = .symbol("exclamationmark.circle")
        case .permissionRequired:
            icon = .symbol("lock.shield")
        }

        return ResultItem(
            id: ResultID(module: module, key: "diagnostic.\(diagnostic.kindTitle)"),
            title: diagnostic.message,
            titleAttributed: AttributedString(diagnostic.message),
            subtitle: diagnostic.kindSubtitle,
            icon: icon,
            primaryAction: Action(
                id: ActionID(module: module, key: "diagnostic"),
                title: "OK",
                kind: .noop
            ),
            rankingHints: RankingHints(basePriority: 100),
            rowKind: .informational
        )
    }
}

private extension ModuleDiagnostic {
    var kindTitle: String {
        switch kind {
        case .timeout: "timeout"
        case .error: "error"
        case .degraded: "degraded"
        case .permissionRequired: "permission"
        }
    }

    var kindSubtitle: String? {
        switch kind {
        case .timeout:
            "Query timed out"
        case .error:
            "Module error"
        case .degraded:
            "Partial results"
        case .permissionRequired:
            "Permission required"
        }
    }
}
