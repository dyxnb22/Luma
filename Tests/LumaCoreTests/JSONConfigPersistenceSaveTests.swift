import Foundation
import LumaCore
import Testing

@Test func jsonConfigPersistenceSaveReplacesExistingAtomically() throws {
    struct Sample: Codable, Equatable {
        var value: Int
    }

    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    let url = dir.appendingPathComponent("config.json")
    try JSONConfigPersistence.save(Sample(value: 1), to: url)
    try JSONConfigPersistence.save(Sample(value: 2), to: url)

    let data = try Data(contentsOf: url)
    let decoded = try JSONDecoder().decode(Sample.self, from: data)
    #expect(decoded.value == 2)
    #expect(!FileManager.default.fileExists(atPath: url.appendingPathExtension("tmp").path))
}
