import Foundation
import Testing

/// Source-level regression guards for module-disable vs panel-hide cancel wiring.
@Test func handleModulesDisabledUsesAsyncWorkCancelOnly() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    #expect(source.contains("func cancelLauncherAsyncWork()"))
    #expect(source.contains("func handleModulesDisabled(removed: Set<ModuleIdentifier>)"))
    guard let range = source.range(of: "func handleModulesDisabled(removed: Set<ModuleIdentifier>)") else {
        Issue.record("handleModulesDisabled not found")
        return
    }
    let bodyStart = range.upperBound
    guard let bodyEnd = source[bodyStart...].range(of: "\n    func activatePanelForQueryApply()") else {
        Issue.record("handleModulesDisabled body end not found")
        return
    }
    let body = String(source[bodyStart..<bodyEnd.lowerBound])
    #expect(body.contains("cancelLauncherAsyncWork()"))
    #expect(!body.contains("cancelActiveQueryAndSnapshotApply()"))
    #expect(!body.contains("isPanelActiveForQueryApply = false"))
}

@Test func cancelActiveQueryAndSnapshotApplyMarksPanelInactive() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let path = root.appending(path: "Sources/LumaApp/Launcher/LauncherRootController.swift")
    let source = try String(contentsOf: path, encoding: .utf8)
    guard let range = source.range(of: "func cancelActiveQueryAndSnapshotApply()") else {
        Issue.record("cancelActiveQueryAndSnapshotApply not found")
        return
    }
    let bodyStart = range.upperBound
    guard let bodyEnd = source[bodyStart...].range(of: "\n    func handleModulesDisabled(removed: Set<ModuleIdentifier>)") else {
        Issue.record("cancelActiveQueryAndSnapshotApply body end not found")
        return
    }
    let body = String(source[bodyStart..<bodyEnd.lowerBound])
    #expect(body.contains("isPanelActiveForQueryApply = false"))
    #expect(body.contains("applySessionEvent(.panelHideBegan)"))
}

@Test func finishPresentationGuardsPresentationGeneration() throws {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let presenterPath = root.appending(path: "Sources/LumaApp/Launcher/Session/LauncherDetailPresenter.swift")
    let source = try String(contentsOf: presenterPath, encoding: .utf8)
    #expect(source.contains("isPresentationGenerationCurrent(generation)"))
    #expect(source.contains("detailLifecycle.nextPresentationGeneration()"))
}
