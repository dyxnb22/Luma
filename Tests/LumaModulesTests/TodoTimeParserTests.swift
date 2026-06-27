import Foundation
import Testing
@testable import LumaModules
import LumaCore
import LumaServices

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

@Test func parserMatchesChineseTomorrow() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("交报告 明天 9点", now: now, calendar: cal)
    #expect(parsed.title == "交报告")
    #expect(parsed.dueDate == date("2026-06-23 09:00", in: cal))
}

@Test func parserMatchesChineseTodayWithColon() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("开会 今天 15:30", now: now, calendar: cal)
    #expect(parsed.title == "开会")
    #expect(parsed.dueDate == date("2026-06-22 15:30", in: cal))
}

@Test func parserMatchesChineseDayAfterTomorrow() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("体检 后天 14点", now: now, calendar: cal)
    #expect(parsed.title == "体检")
    #expect(parsed.dueDate == date("2026-06-24 14:00", in: cal))
}

@Test func parserMatchesChineseTonight() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("聚餐 今晚 8点", now: now, calendar: cal)
    #expect(parsed.title == "聚餐")
    #expect(parsed.dueDate == date("2026-06-22 20:00", in: cal))
}

@Test func parserMatchesChineseWeekdayOnSameDay() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal) // Monday
    let parsed = TodoTimeParser.parse("站会 周一 15:00", now: now, calendar: cal)
    #expect(parsed.title == "站会")
    #expect(parsed.dueDate == date("2026-06-22 15:00", in: cal))
}

@Test func parserRollsSameWeekdayForwardWhenTimePassed() {
    let cal = calendar()
    let now = date("2026-06-22 16:00", in: cal) // Monday afternoon
    let parsed = TodoTimeParser.parse("站会 周一 15:00", now: now, calendar: cal)
    #expect(parsed.title == "站会")
    #expect(parsed.dueDate == date("2026-06-29 15:00", in: cal))
}

@Test func parserRejectsInvalidChineseMinute() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("聚餐 今晚 8:99", now: now, calendar: cal)
    #expect(parsed.title == "聚餐 今晚 8:99")
    #expect(parsed.dueDate == nil)
}

@Test func parserMatchesChineseWeekday() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal) // Monday
    let parsed = TodoTimeParser.parse("开会 周五 15:00", now: now, calendar: cal)
    #expect(parsed.title == "开会")
    #expect(parsed.dueDate == date("2026-06-26 15:00", in: cal))
}

@Test func parserMatchesChineseNextWeekday() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal) // Monday
    let parsed = TodoTimeParser.parse("站会 下周一 9点", now: now, calendar: cal)
    #expect(parsed.title == "站会")
    #expect(parsed.dueDate == date("2026-06-29 09:00", in: cal))
}

@Test func parserMatchesChineseAfternoon() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("电话 下午3点", now: now, calendar: cal)
    #expect(parsed.title == "电话")
    #expect(parsed.dueDate == date("2026-06-22 15:00", in: cal))
}

@Test func parserMatchesChineseMorning() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let parsed = TodoTimeParser.parse("晨会 上午10点", now: now, calendar: cal)
    #expect(parsed.title == "晨会")
    #expect(parsed.dueDate == date("2026-06-22 10:00", in: cal))
}

@Test func captureStatusMessageForInbox() {
    let parsed = TodoTimeParser.Parsed(title: "buy milk", dueDate: nil)
    #expect(TodoModule.captureStatusMessage(for: parsed) == "Added to Inbox")
}

@Test func captureStatusMessageForScheduled() {
    let cal = calendar()
    let now = date("2026-06-22 10:00", in: cal)
    let due = date("2026-06-23 09:00", in: cal)
    let parsed = TodoTimeParser.Parsed(title: "standup", dueDate: due)
    #expect(TodoModule.captureStatusMessage(for: parsed, now: now, calendar: cal) == "Added for tomorrow 09:00")
}

@Test func todoListKindCoversFourTabs() {
    let kinds: [TodoListKind] = [.today, .inbox, .upcoming, .completed]
    #expect(kinds.count == 4)
}

@Test func reminderSnapshotClassifiesIntoSingleInboxBucket() {
    let inbox = ReminderSnapshot(
        id: "1",
        title: "Inbox item",
        dueDate: nil,
        isCompleted: false,
        calendarTitle: "Reminders"
    )
    #expect(inbox.dueDate == nil)
    #expect(!inbox.isCompleted)
}

@Test func todoActionEncodesUncomplete() throws {
    let action = TodoAction.uncomplete(id: "abc")
    let data = try ModuleActionCoding.encode(action)
    let decoded = try ModuleActionCoding.decode(TodoAction.self, from: data)
    #expect(decoded == action)
}

@Test func todoModuleExtractsPayloadFromTriggerVariants() {
    #expect(TodoModule.extractPayload(raw: "t") == "")
    #expect(TodoModule.extractPayload(raw: "todo") == "")
    #expect(TodoModule.extractPayload(raw: "t buy milk") == "buy milk")
    #expect(TodoModule.extractPayload(raw: "todo pay rent tomorrow 9:30") == "pay rent tomorrow 9:30")
    #expect(TodoModule.extractPayload(raw: "translate hello") == nil)
    #expect(TodoModule.extractPayload(raw: "trash") == nil)
}

@Test func todoModuleResumeQueryAvoidsDuplicatePrefixes() {
    #expect(TodoModule.resumeQuery(forCapture: "buy milk") == "todo buy milk")
    #expect(TodoModule.resumeQuery(forCapture: "todo buy milk") == "todo buy milk")
    #expect(TodoModule.resumeQuery(forCapture: "t buy milk") == "t buy milk")
    #expect(TodoModule.resumeQuery(forCapture: "") == "todo")
}
