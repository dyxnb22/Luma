import Foundation
import LumaCore
import Testing

@Test func jsonConfigPersistenceQuarantinesCorruptFile() throws {
    ConfigCorruptionRegistry.resetForTesting()
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    let url = dir.appendingPathComponent("test.json")
    try "{ not json".write(to: url, atomically: true, encoding: .utf8)

    struct Sample: Codable, Equatable { var value: Int = 0 }
    let result = JSONConfigPersistence.load(from: url, fallback: Sample())
    #expect(result.wasCorrupt)
    #expect(result.value == Sample())
    #expect(!FileManager.default.fileExists(atPath: url.path))
    #expect(ConfigCorruptionRegistry.snapshot().contains("test.json"))
}
