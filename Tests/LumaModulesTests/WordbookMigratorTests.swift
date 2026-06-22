import Foundation
import Testing
@testable import LumaModules

private func tempDir() -> URL {
    FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-wordbook-tests", isDirectory: true)
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
}

@Test func migratorCopiesSourceToDestinationWhenAbsent() throws {
    let dir = tempDir()
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }

    let source = dir.appendingPathComponent("source.sqlite3")
    let destination = dir.appendingPathComponent("dest/wordpet.sqlite3")
    try Data("payload".utf8).write(to: source)

    let result = try WordbookMigrator.migrateIfNeeded(source: source, destination: destination)

    if case .copied = result {} else {
        Issue.record("Expected .copied result, got \(result)")
    }
    #expect(FileManager.default.fileExists(atPath: destination.path))
}

@Test func migratorIsIdempotentWhenDestinationExists() throws {
    let dir = tempDir()
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }

    let source = dir.appendingPathComponent("source.sqlite3")
    let destination = dir.appendingPathComponent("wordpet.sqlite3")
    try Data("source".utf8).write(to: source)
    try Data("existing".utf8).write(to: destination)

    let result = try WordbookMigrator.migrateIfNeeded(source: source, destination: destination)
    #expect(result == .alreadyMigrated)

    let preserved = try String(contentsOf: destination, encoding: .utf8)
    #expect(preserved == "existing")
}

@Test func migratorReportsMissingSourceWithoutThrowing() throws {
    let dir = tempDir()
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }

    let source = dir.appendingPathComponent("never-existed.sqlite3")
    let destination = dir.appendingPathComponent("wordpet.sqlite3")

    let result = try WordbookMigrator.migrateIfNeeded(source: source, destination: destination)
    #expect(result == .sourceMissing)
    #expect(!FileManager.default.fileExists(atPath: destination.path))
}
