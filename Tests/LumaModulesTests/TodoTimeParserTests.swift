import Foundation
import Testing
@testable import LumaModules

private func calendar(timeZone identifier: String = "Asia/Shanghai") -> Calendar {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(identifier: identifier) ?? .current
    return calendar
}

private func date(_ string: String, in calendar: Calendar) -> Date {
    let formatter = DateFormatter()
    formatter.calendar = calendar
    formatter.timeZone = calendar.timeZone
    formatter.dateFormat = "yyyy-MM-dd HH:mm"
    return formatter.date(from: string)!
}

@Test func parserReturnsTitleWhenNoSuffix() {
    let parsed = TodoTimeParser.parse("buy milk")
    #expect(parsed.title == "buy milk")
    #expect(parsed.dueDate == nil)
}

@Test func parserMatchesPlusMinutes() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("call dentist +30m", now: now, calendar: cal)
    #expect(parsed.title == "call dentist")
    #expect(parsed.dueDate == now.addingTimeInterval(30 * 60))
}

@Test func parserMatchesPlusHours() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("review PR +2h", now: now, calendar: cal)
    #expect(parsed.title == "review PR")
    #expect(parsed.dueDate == now.addingTimeInterval(2 * 60 * 60))
}

@Test func parserMatchesToday() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("standup today 15:00", now: now, calendar: cal)
    #expect(parsed.title == "standup")
    #expect(parsed.dueDate == date("2026-06-22 15:00", in: cal))
}

@Test func parserMatchesTomorrow() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("send invoice tomorrow 9", now: now, calendar: cal)
    #expect(parsed.title == "send invoice")
    #expect(parsed.dueDate == date("2026-06-23 09:00", in: cal))
}

@Test func parserRejectsOutOfRangeHour() {
    let parsed = TodoTimeParser.parse("nope today 27:00")
    #expect(parsed.title == "nope today 27:00")
    #expect(parsed.dueDate == nil)
}

@Test func todoModuleExtractsPayloadFromTriggerVariants() {
    #expect(TodoModule.extractPayload(raw: "t") == "")
    #expect(TodoModule.extractPayload(raw: "todo") == "")
    #expect(TodoModule.extractPayload(raw: "t buy milk") == "buy milk")
    #expect(TodoModule.extractPayload(raw: "todo pay rent tomorrow 9:30") == "pay rent tomorrow 9:30")
    #expect(TodoModule.extractPayload(raw: "translate hello") == nil)
    #expect(TodoModule.extractPayload(raw: "trash") == nil)
}
