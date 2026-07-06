import Foundation
import LumaCore
import LumaInfrastructure
import Testing

@Test func crashLogBufferRedactsAtWrite() async {
    await CrashLogBuffer.shared.record("query=secret clipboard=hidden")
    let entries = await CrashLogBuffer.shared.all()
    let combined = entries.joined(separator: "\n")
    #expect(!combined.contains("secret"))
    #expect(!combined.contains("hidden"))
    #expect(combined.contains("<redacted>"))
}

@Test func crashLogBufferRedactsPathHeuristicsAtWrite() async {
    await CrashLogBuffer.shared.record("note saved ~/Projects/Luma/secret.swift")
    let entries = await CrashLogBuffer.shared.all()
    let combined = entries.joined(separator: "\n")
    #expect(!combined.contains("~/Projects"))
    #expect(combined.contains("~/<redacted>"))
}
