import Foundation
import Testing
@testable import LumaModules

@Test func notesImageToolsMigratesNestedNoteReferenceToAssetsRelatively() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }

    let project = root.appendingPathComponent("Project", isDirectory: true)
    let images = project.appendingPathComponent("images", isDirectory: true)
    try FileManager.default.createDirectory(at: images, withIntermediateDirectories: true)

    let note = project.appendingPathComponent("Plan.md")
    let image = images.appendingPathComponent("diagram.png")
    try Data([0x89, 0x50, 0x4E, 0x47]).write(to: image)
    try "![diagram](images/diagram.png)\n![remote](https://example.com/image.png)".write(to: note, atomically: true, encoding: .utf8)

    let tools = NotesImageTools(root: root)
    let result = try await tools.migrateToAssets()

    #expect(result.moved == 1)
    let rewritten = try String(contentsOf: note, encoding: .utf8)
    #expect(rewritten.contains("![diagram](../_assets/diagram.png)"))
    #expect(rewritten.contains("https://example.com/image.png"))
    #expect(FileManager.default.fileExists(atPath: root.appendingPathComponent("_assets/diagram.png").path))
}
