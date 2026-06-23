import Foundation
import LumaModules

enum ClipboardDisplayRow: Equatable {
    case header(String)
    case entry(ClipboardEntry)
}

enum ClipboardTimeGrouping {
    static func displayRows(for entries: [ClipboardEntry], now: Date = Date()) -> [ClipboardDisplayRow] {
        var rows: [ClipboardDisplayRow] = []
        var lastHeader: String?
        let calendar = Calendar.current
        for entry in entries {
            let header = bucketLabel(for: entry.createdAt, now: now, calendar: calendar)
            if header != lastHeader {
                rows.append(.header(header))
                lastHeader = header
            }
            rows.append(.entry(entry))
        }
        return rows
    }

    static func bucketLabel(for date: Date, now: Date, calendar: Calendar) -> String {
        if calendar.isDateInToday(date) { return "Today" }
        if calendar.isDateInYesterday(date) { return "Yesterday" }
        let start = calendar.startOfDay(for: date)
        let today = calendar.startOfDay(for: now)
        if let days = calendar.dateComponents([.day], from: start, to: today).day, days < 7 {
            return "7 Days"
        }
        return "Earlier"
    }
}
