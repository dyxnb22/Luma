import XCTest
@testable import LumaWorkbenchCore

final class TerminalControlFilterTests: XCTestCase {
    func testKeepsCsiAndPlainText() {
        var filter = TerminalControlFilter()
        let input = Array("hello\u{1b}[31m red".utf8)
        XCTAssertEqual(filter.filter(input[...]), input)
    }

    func testDropsOscAcrossChunksWithoutEmittingClipboardQuery() {
        var filter = TerminalControlFilter()
        XCTAssertEqual(filter.filter(Array("before\u{1b}]52;c;?".utf8)[...]), Array("before".utf8))
        XCTAssertEqual(filter.filter(Array("\u{07}after".utf8)[...]), Array("after".utf8))
    }

    func testDropsApcAndRecoversAfterStringTerminator() {
        var filter = TerminalControlFilter()
        let input = Array("a\u{1b}_payload\u{1b}\\b".utf8)
        XCTAssertEqual(filter.filter(input[...]), Array("ab".utf8))
    }

    func testDropsAllUnsupportedSevenBitStringControls() {
        for introducer in [UInt8(0x50), 0x58, 0x5e, 0x5f] { // DCS, SOS, PM, APC
            var filter = TerminalControlFilter()
            let input = [UInt8(0x61), 0x1b, introducer, 0x78, 0x1b, 0x5c, 0x62]
            XCTAssertEqual(filter.filter(input[...]), Array("ab".utf8))
        }
    }

    func testNonOscStringsDoNotTreatBellAsTheirTerminator() {
        var filter = TerminalControlFilter()
        let input = [
            UInt8(0x61), 0x1b, 0x50, 0x78, 0x07, 0x79, 0x1b, 0x5c, 0x62,
        ]
        XCTAssertEqual(filter.filter(input[...]), Array("ab".utf8))
    }

    func testDropsEightBitOscAndItsStringTerminator() {
        var filter = TerminalControlFilter()
        XCTAssertEqual(
            filter.filter([0x9d, 0x35, 0x32, 0x3b, 0x63, 0x3b, 0x3f, 0x9c, 0x6f, 0x6b][...]),
            [0x6f, 0x6b]
        )
    }

    func testKeepsUtf8ContinuationByteThatMatchesEightBitOsc() {
        var filter = TerminalControlFilter()
        let input = Array("数据保留期限".utf8)
        XCTAssertEqual(filter.filter(input[...]), input)
    }

    func testKeepsUtf8AcrossPtyChunkBoundaryBeforeC1LookingByte() {
        var filter = TerminalControlFilter()
        // 保 is E4 BF 9D. Its final byte must not start an 8-bit OSC string.
        XCTAssertEqual(filter.filter([0xe4, 0xbf][...]), [0xe4, 0xbf])
        XCTAssertEqual(filter.filter([0x9d, 0x61][...]), [0x9d, 0x61])
    }

    func testKeepsNormalUtf8AcrossEveryTwoChunkBoundary() {
        let input = Array("English 中文 🙂 e\u{301}".utf8)
        for split in 0...input.count {
            var filter = TerminalControlFilter()
            let first = filter.filter(input[..<split])
            let second = filter.filter(input[split...])
            XCTAssertEqual(first + second, input, "split at byte \(split)")
        }
    }

    func testUnterminatedStringDoesNotRetainPayload() {
        var filter = TerminalControlFilter()
        XCTAssertEqual(filter.filter([0x1b, 0x5d][...]), [])
        for _ in 0..<4 {
            XCTAssertEqual(filter.filter(Array(repeating: 0x78, count: 1024)[...]), [])
        }
        XCTAssertEqual(filter.filter(Array("\u{07}ok".utf8)[...]), Array("ok".utf8))
    }

    func testResetDropsUnterminatedStateFromThePreviousPtySession() {
        var filter = TerminalControlFilter()
        XCTAssertEqual(
            filter.filter(Array("\u{1b}]52;c;old-session".utf8)[...]),
            []
        )

        filter.reset()

        let nextSession = Array("new session 中文".utf8)
        XCTAssertEqual(filter.filter(nextSession[...]), nextSession)
    }

    func testResetDropsDanglingEscapeAndUtf8StateFromThePreviousPtySession() {
        var filter = TerminalControlFilter()
        XCTAssertEqual(filter.filter([0x1b][...]), [])
        filter.reset()
        XCTAssertEqual(filter.filter(Array("[new".utf8)[...]), Array("[new".utf8))

        XCTAssertEqual(filter.filter([0xe4][...]), [0xe4])
        filter.reset()
        // 0x9d is now an independent C1 OSC rather than a continuation from the old session.
        XCTAssertEqual(filter.filter([0x9d, 0x78, 0x9c, 0x6f, 0x6b][...]), Array("ok".utf8))
    }
}
