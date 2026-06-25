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
                "t ? — this help",
                "t buy milk — create reminder (Inbox when no date)",
                "t pay rent tomorrow 9:00 — create with due date",
                "Return on a row — mark complete",
                "Detail panel — Today / Inbox / Upcoming / Done tabs",
                "Edit — Today, Tomorrow, Clear Date, or undo in Done"
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
                "rec — recent items + open logbook",
                "rec log — full Records view",
                "rec 三体 book done 9 #sci-fi — quick capture DSL",
                "rec 三体 — search or partial capture",
                "m / media / record / log — same triggers"
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
                "n — recent notes",
                "n <query> — fuzzy find by filename",
                "n new <title> — create in Inbox and open",
                "n new <template> <title> — create from template",
                "n daily — open or create today's daily note",
                "n review week — weekly review with modified notes",
                "n doctor — vault health check",
                "tag:swift / type:reading — filter by metadata",
                "note … — same commands (legacy alias)"
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
