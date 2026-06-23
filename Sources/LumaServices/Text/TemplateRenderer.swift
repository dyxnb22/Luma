import Foundation

public enum TemplateRenderer {
    public static func render(
        _ content: String,
        title: String,
        now: Date = Date(),
        calendar: Calendar = .current
    ) -> String {
        let isoDate = isoDateString(from: now, calendar: calendar)
        let mediumDate = mediumDateString(from: now)
        let week = isoWeekString(from: now, calendar: calendar)

        var result = content
        result = result.replacingOccurrences(of: "{{title}}", with: title)
        result = result.replacingOccurrences(of: "{{date}}", with: isoDate)
        result = result.replacingOccurrences(of: "{{date|medium}}", with: mediumDate)
        result = result.replacingOccurrences(of: "{{week}}", with: week)
        return result
    }

    private static func isoDateString(from date: Date, calendar: Calendar) -> String {
        let components = calendar.dateComponents([.year, .month, .day], from: date)
        guard let year = components.year, let month = components.month, let day = components.day else {
            return ISO8601DateFormatter().string(from: date).prefix(10).description
        }
        return String(format: "%04d-%02d-%02d", year, month, day)
    }

    private static func mediumDateString(from date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .none
        return formatter.string(from: date)
    }

    private static func isoWeekString(from date: Date, calendar: Calendar) -> String {
        let components = calendar.dateComponents([.yearForWeekOfYear, .weekOfYear], from: date)
        guard let year = components.yearForWeekOfYear, let week = components.weekOfYear else { return "" }
        return String(format: "%04d-W%02d", year, week)
    }
}
