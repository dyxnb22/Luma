import Foundation
import Testing

private let appKitPropertyOverridePattern = try! NSRegularExpression(
    pattern: #"^\s*override (class )?var (isFlipped|acceptsFirstResponder|isOpaque|intrinsicContentSize|canBecomeKey|canBecomeMain|string|stringValue)\b"#
)

@Test func appKitPropertyOverridePatternFlagsMissingNonisolated() {
    let badLine = "    override var isFlipped: Bool { true }"
    let goodLine = "    nonisolated override var isFlipped: Bool { true }"
    let badRange = NSRange(badLine.startIndex..., in: badLine)
    #expect(appKitPropertyOverridePattern.firstMatch(in: badLine, range: badRange) != nil)
    #expect(goodLine.contains("nonisolated"))
}

@Test func appKitExecutorBoundaryScanPasses() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let script = root.appending(path: "scripts/scan_appkit_executor_risk.sh")
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/bin/bash")
    process.arguments = [script.path()]
    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = pipe
    try process.run()
    process.waitUntilExit()
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(decoding: data, as: UTF8.self)
  #expect(process.terminationStatus == 0, "scan failed:\n\(output)")
}
