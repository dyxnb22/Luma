import Foundation
import LumaCore
import LumaModules

/// Default capture implementation: builds drafts, writes resume/activity, no module warmup.
struct DefaultWorkbenchCaptureService: WorkbenchCaptureService {
    func capture(
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        context: WorkbenchContext
    ) async -> WorkbenchCaptureResult? {
        WorkbenchCaptureEngine.capture(source: source, target: target, context: context)
    }

    func applyResult(
        _ result: WorkbenchCaptureResult,
        context: WorkbenchContext,
        attribution: WorkbenchCaptureAttribution
    ) async {
        var resume = LauncherResumeStore.load()
        switch result.target {
        case .snippetDraft, .projectSnippetDraft:
            resume.snippetDraftJSON = result.resumeDraftJSON
        case .quicklinkDraft:
            resume.quicklinkDraftJSON = result.resumeDraftJSON
        case .todoDraft:
            resume.todoCaptureText = result.preview
        case .noteDraft:
            break
        }
        LauncherResumeStore.save(resume)

        if let project = context.currentProject {
            let identity = ProjectIdentity(context: project)
            let entry = await WorkbenchActivityStore.shared.recordCapture(
                result: result,
                context: context,
                attribution: attribution
            )
            await WorkbenchLinkStore.shared.recordLink(for: entry, identity: identity)
        } else {
            await WorkbenchActivityStore.shared.recordCapture(
                result: result,
                context: context,
                attribution: attribution
            )
        }
    }

    @MainActor
    func stagePendingState(for result: WorkbenchCaptureResult) {
        guard let data = result.resumeDraftJSON else { return }
        switch result.target {
        case .snippetDraft, .projectSnippetDraft:
            if let draft = try? JSONDecoder().decode(SnippetDraft.self, from: data) {
                LauncherSharedState.pendingSnippetDraft = draft
            }
        case .quicklinkDraft:
            if let draft = try? JSONDecoder().decode(URLQuicklinkDraft.self, from: data) {
                LauncherSharedState.pendingQuicklinkDraft = draft
            }
        case .todoDraft, .noteDraft:
            break
        }
    }
}
