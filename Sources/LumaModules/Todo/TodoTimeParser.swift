import Foundation

/// Parses an optional trailing time suffix off a Todo capture string.
///
/// Supported suffixes (case insensitive, trailing):
/// - `+30m` `+2h` `+45min`
/// - `today 15:00` `today 9:30`
/// - `tomorrow 15:00` `tomorrow 9`
/// - Chinese: `今天 15:00` `明天 9点` `后天 14:30`
///
/// Anything else stays as part of the title. Natural-language parsing is intentionally out of scope
/// for v0.1 (see ADR-009).
public enum TodoTimeParser {
    public struct Parsed: Sendable, Equatable {
        public let title: String
        public let dueDate: Date?
    }

    public static func parse(_ raw: String, now: Date = Date(), calendar: Calendar = .current) -> Parsed {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return Parsed(title: "", dueDate: nil) }

        if let relative = matchRelative(trimmed, now: now) {
            return relative
        }
        if let absolute = matchAbsolute(trimmed, now: now, calendar: calendar) {
            return absolute
        }
        if let chinese = matchChineseAbsolute(trimmed, now: now, calendar: calendar) {
            return chinese
        }
        return Parsed(title: trimmed, dueDate: nil)
    }

    // MARK: - Patterns

    private static let relativeRegex = try? NSRegularExpression(
        pattern: #"^(.+?)\s+\+(\d+)\s*(m|min|mins|minute|minutes|h|hr|hrs|hour|hours)$"#,
        options: [.caseInsensitive]
    )

    private static let absoluteRegex = try? NSRegularExpression(
        pattern: #"^(.+?)\s+(today|tomorrow)\s+(\d{1,2})(?::(\d{2}))?$"#,
        options: [.caseInsensitive]
    )

    private static let chineseAbsoluteRegex = try? NSRegularExpression(
        pattern: #"^(.+?)\s+(今天|明天|后天)\s*(\d{1,2})(?:[:：](\d{2})|点(\d{1,2})?|点)?$"#
    )

    private static func matchRelative(_ text: String, now: Date) -> Parsed? {
        guard let regex = relativeRegex else { return nil }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = regex.firstMatch(in: text, range: range), match.numberOfRanges == 4 else { return nil }
        guard let titleRange = Range(match.range(at: 1), in: text),
              let amountRange = Range(match.range(at: 2), in: text),
              let unitRange = Range(match.range(at: 3), in: text) else { return nil }

        let title = String(text[titleRange]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let amount = Int(text[amountRange]) else { return nil }
        let unit = text[unitRange].lowercased()
        let seconds: TimeInterval
        if unit.hasPrefix("h") {
            seconds = TimeInterval(amount) * 60 * 60
        } else {
            seconds = TimeInterval(amount) * 60
        }
        return Parsed(title: title, dueDate: now.addingTimeInterval(seconds))
    }

    private static func matchAbsolute(_ text: String, now: Date, calendar: Calendar) -> Parsed? {
        guard let regex = absoluteRegex else { return nil }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = regex.firstMatch(in: text, range: range) else { return nil }
        guard let titleRange = Range(match.range(at: 1), in: text),
              let dayRange = Range(match.range(at: 2), in: text),
              let hourRange = Range(match.range(at: 3), in: text) else { return nil }
        let minuteRange = Range(match.range(at: 4), in: text)

        let title = String(text[titleRange]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let hour = Int(text[hourRange]), (0...23).contains(hour) else { return nil }
        var minute = 0
        if let minuteRange {
            guard let parsed = Int(text[minuteRange]), (0...59).contains(parsed) else { return nil }
            minute = parsed
        }

        let dayWord = text[dayRange].lowercased()
        var dayOffset = 0
        if dayWord == "tomorrow" {
            dayOffset = 1
        }

        return makeAbsolute(title: title, dayOffset: dayOffset, hour: hour, minute: minute, now: now, calendar: calendar)
    }

    private static func matchChineseAbsolute(_ text: String, now: Date, calendar: Calendar) -> Parsed? {
        guard let regex = chineseAbsoluteRegex else { return nil }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = regex.firstMatch(in: text, range: range) else { return nil }
        guard let titleRange = Range(match.range(at: 1), in: text),
              let dayRange = Range(match.range(at: 2), in: text),
              let hourRange = Range(match.range(at: 3), in: text) else { return nil }

        let title = String(text[titleRange]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let hour = Int(text[hourRange]), (0...23).contains(hour) else { return nil }

        var minute = 0
        if let colonRange = Range(match.range(at: 4), in: text), !colonRange.isEmpty {
            guard let parsed = Int(text[colonRange]), (0...59).contains(parsed) else { return nil }
            minute = parsed
        } else if let minuteRange = Range(match.range(at: 5), in: text), !minuteRange.isEmpty {
            guard let parsed = Int(text[minuteRange]), (0...59).contains(parsed) else { return nil }
            minute = parsed
        }

        let dayWord = String(text[dayRange])
        let dayOffset: Int
        switch dayWord {
        case "明天": dayOffset = 1
        case "后天": dayOffset = 2
        default: dayOffset = 0
        }

        return makeAbsolute(title: title, dayOffset: dayOffset, hour: hour, minute: minute, now: now, calendar: calendar)
    }

    private static func makeAbsolute(
        title: String,
        dayOffset: Int,
        hour: Int,
        minute: Int,
        now: Date,
        calendar: Calendar
    ) -> Parsed? {
        guard let baseDay = calendar.date(byAdding: .day, value: dayOffset, to: calendar.startOfDay(for: now)) else {
            return nil
        }
        var components = calendar.dateComponents([.year, .month, .day], from: baseDay)
        components.hour = hour
        components.minute = minute
        guard let date = calendar.date(from: components) else { return nil }
        return Parsed(title: title, dueDate: date)
    }
}
