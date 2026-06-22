import Foundation
import Testing
@testable import LumaCore

@Test func corruptJSONIsQuarantinedAndTrackerStartsEmpty() async throws {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    let url = dir.appendingPathComponent("app-activations.json")
    try "not json at all".write(to: url, atomically: true, encoding: .utf8)

    let tracker = AppActivationTracker(url: url)
    let ranked = await tracker.rankedBundleIDs(from: ["com.example"])
    #expect(ranked == ["com.example"])

    let backups = try FileManager.default.contentsOfDirectory(atPath: dir.path).filter { $0.contains(".corrupt-") }
    #expect(backups.count == 1)
}
