import Foundation
import LumaCore

public enum ModuleHelp {
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

    private static func lines(for module: ModuleIdentifier) -> [String] {
        switch module.rawValue {
        case "todo":
            return [
                "t — list today's due reminders",
                "t buy milk — create reminder",
                "t pay rent tomorrow 9:00 — create with due date",
                "Return on a row — mark complete"
            ]
        case "snippets":
            return [
                "s — top snippets by frecency",
                "s git — fuzzy search title/tags/content",
                "Return — copy snippet (supports {{date}}, {{clipboard}})",
                "Tab — paste into front app"
            ]
        case "media":
            return [
                "m — recent items + open log",
                "m log — full media log view",
                "m dune movie done 9 — quick capture DSL",
                "m dune — search or partial capture"
            ]
        case "wordbook":
            return [
                "word — today's due words + start review",
                "word abandon — search term/meaning",
                "Start Review — opens review panel"
            ]
        case "secrets":
            return [
                "secret unlock — unlock vault",
                "secret aws — search by label",
                "Return — copy secret (auto-clear pasteboard)"
            ]
        case "notes":
            return [
                "Type filename — fuzzy find notes",
                "Return — open in Typora",
                "note backlinks <name> — find [[wiki]] references"
            ]
        case "clipboard":
            return [
                "clip — search clipboard history",
                "clip https — filter links"
            ]
        case "translate":
            return [
                "translate <text> — translate to target language",
                "Open card for language settings"
            ]
        case "apps":
            return [
                "Type app name — launch or focus",
                "app top — memory usage leaders (quit from row)"
            ]
        case "events":
            return [
                "e — list today's calendar events",
                "e meet john tomorrow 14:00 — create event",
                "Return on capture row — save to Calendar"
            ]
        case "commands":
            return [
                "open-settings — Luma preferences",
                "reload-modules — refresh module registry",
                "quit — exit Luma"
            ]
        default:
            return ["No help available for this module."]
        }
    }
}
