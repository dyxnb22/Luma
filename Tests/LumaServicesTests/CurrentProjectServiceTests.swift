import Foundation
import LumaCore
import Testing
@testable import LumaServices

private struct TestProjectMatcher: ProjectMatcherClient {
    let matches: [String: MatchedProject]

    func match(label: String) async -> MatchedProject? {
        matches[label]
    }
}

@Suite struct CurrentProjectServiceTests {
    @Test func applyProjectMatchClearsStalePathForUnknownLabel() async {
        let service = CurrentProjectService(matcher: TestProjectMatcher(matches: [
            "Luma": MatchedProject(path: "/Users/dev/Luma", name: "Luma")
        ]))

        let raw = CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.todesktop.test",
            windowTitle: "Other — Cursor",
            projectLabel: "Other",
            filename: nil,
            matchedProjectPath: "/Users/dev/Luma",
            matchedProjectName: "Luma"
        )

        let resolved = await service.applyProjectMatch(to: raw)
        #expect(resolved.matchedProjectPath == nil)
        #expect(resolved.matchedProjectName == nil)
    }

    @Test func applyProjectMatchResolvesKnownLabel() async {
        let service = CurrentProjectService(matcher: TestProjectMatcher(matches: [
            "Luma": MatchedProject(path: "/Users/dev/Luma", name: "Luma")
        ]))

        let raw = CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.todesktop.test",
            windowTitle: "App.swift — Luma — Cursor",
            projectLabel: "Luma",
            filename: "App.swift"
        )

        let resolved = await service.applyProjectMatch(to: raw)
        #expect(resolved.matchedProjectPath == "/Users/dev/Luma")
        #expect(resolved.matchedProjectName == "Luma")
    }

    @MainActor
    @Test func bootstrapReplacesSharedInstanceBeforeSnapshots() async {
        defer {
            CurrentProjectService.bootstrap(matcher: NoopProjectMatcherClient())
        }

        CurrentProjectService.bootstrap(matcher: TestProjectMatcher(matches: [
            "Luma": MatchedProject(path: "/Users/dev/Luma", name: "Luma")
        ]))

        let resolved = await CurrentProjectService.shared.applyProjectMatch(to: CurrentProjectContext(
            frontAppName: "Cursor",
            bundleID: "com.todesktop.test",
            windowTitle: "Luma — Cursor",
            projectLabel: "Luma",
            filename: nil
        ))
        #expect(resolved.matchedProjectPath == "/Users/dev/Luma")
    }
}
