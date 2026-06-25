import Foundation
import LumaCore

public enum ModuleHelp {
    private static let registry = BuiltInCommandRegistry.make()

    public static func results(for module: ModuleIdentifier) -> [ResultItem] {
        let lines = lines(for: module)
        return lines.enumerated().map { index, line in
            ResultItem(
                id: ResultID(module: module, key: "help.\(index)"),
                title: line,
                titleAttributed: AttributedString(line),
                subtitle: nil,
                icon: .symbol("questionmark.circle"),
                primaryAction: Action(
                    id: ActionID(module: module, key: "help.\(index)"),
                    title: "Help",
                    kind: .noop
                ),
                rankingHints: RankingHints(basePriority: 0)
            )
        }
    }

    public static func isHelpQuery(_ text: String) -> Bool {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return t == "?" || t == "help"
    }

    public static func lines(for module: ModuleIdentifier) -> [String] {
        if let command = registry.command(forModule: module), !command.helpLines.isEmpty {
            return command.helpLines
        }
        return ["No help available for this module."]
    }
}
