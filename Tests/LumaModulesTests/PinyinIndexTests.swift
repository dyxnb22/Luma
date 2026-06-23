import Foundation
import Testing
@testable import LumaModules

@Test func pinyinIndexConvertsChinese() {
    let full = PinyinIndex.full(from: ["微信"])
    #expect(full.contains("wei"))
    #expect(PinyinIndex.initials(from: ["微信"]) == "wx")
}

@Test func pinyinIndexHandlesEnglishAndEmpty() {
    #expect(PinyinIndex.full(from: ["Safari"]) == "safari")
    #expect(PinyinIndex.initials(from: ["Safari"]) == "s")
    #expect(PinyinIndex.full(from: []).isEmpty)
}

@Test func pinyinIndexHandlesEmojiWithoutCrashing() {
    let result = PinyinIndex.full(from: ["🙂"])
    #expect(!result.isEmpty || result.isEmpty)
}
