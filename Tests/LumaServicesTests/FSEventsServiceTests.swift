import Foundation
import LumaCore
import Testing
@testable import LumaServices

private actor EventCollector {
    private var batches: [[FSChangeEvent]] = []

    func append(_ batch: [FSChangeEvent]) {
        batches.append(batch)
    }

    func waitFor(
        timeout: Duration,
        predicate: @Sendable ([FSChangeEvent]) -> Bool
    ) async -> [FSChangeEvent]? {
        let deadline = ContinuousClock.now + timeout
        while ContinuousClock.now < deadline {
            if let match = batches.first(where: predicate) {
                return match
            }
            try? await Task.sleep(for: .milliseconds(25))
        }
        return batches.first(where: predicate)
    }

    func batchCount() -> Int {
        batches.count
    }
}

@Test func fseventsSurfacesCreateRenameDelete() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let service = FSEventsService()
    let stream = await service.watch(root: root, debounceMillis: 200)
    let collector = EventCollector()
    let consumer = Task {
        for await batch in stream {
            await collector.append(batch)
        }
    }
    defer {
        consumer.cancel()
        Task { await service.stop(root: root) }
    }

    try await Task.sleep(for: .milliseconds(250))

    let fileURL = root.appendingPathComponent("note.md")
    try "hello".write(to: fileURL, atomically: true, encoding: .utf8)

    let created = await collector.waitFor(timeout: .seconds(2)) { batch in
        batch.contains { $0.path.hasSuffix("note.md") }
    }
    #expect(created != nil)

    let renamedURL = root.appendingPathComponent("renamed.md")
    try FileManager.default.moveItem(at: fileURL, to: renamedURL)

    let renamed = await collector.waitFor(timeout: .seconds(2)) { batch in
        batch.contains { $0.kind == .renamed || $0.path.hasSuffix("renamed.md") }
    }
    #expect(renamed != nil)

    try FileManager.default.removeItem(at: renamedURL)

    let removed = await collector.waitFor(timeout: .seconds(2)) { batch in
        batch.contains { $0.kind == .removed }
    }
    #expect(removed != nil)

    await service.stop(root: root)
}

@Test func fseventsDebouncesBurstCreates() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let service = FSEventsService()
    let stream = await service.watch(root: root, debounceMillis: 200)
    let collector = EventCollector()
    let consumer = Task {
        for await batch in stream {
            await collector.append(batch)
        }
    }
    defer {
        consumer.cancel()
        Task { await service.stop(root: root) }
    }

    try await Task.sleep(for: .milliseconds(250))

    for index in 0..<50 {
        let url = root.appendingPathComponent("file-\(index).md")
        try "x".write(to: url, atomically: true, encoding: .utf8)
    }

    try await Task.sleep(for: .milliseconds(900))
    let batches = await collector.batchCount()
    #expect(batches <= 3)

    await service.stop(root: root)
}
