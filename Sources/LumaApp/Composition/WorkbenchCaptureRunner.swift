import Foundation
import LumaCore
import LumaModules

/// Runs capture + resume/activity writes before opening detail or executing follow-up actions.
struct WorkbenchCaptureRunner {
    private let captureService = DefaultWorkbenchCaptureService()
    private let contextBuilder = WorkbenchContextBuilder()

    func runCapture(
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        enabledModuleIDs: Set<ModuleIdentifier>,
        pinnedModuleIDs: Set<ModuleIdentifier>,
        clipboardPreview: String?,
        selectionText: String?,
        attribution: WorkbenchCaptureAttribution
    ) async -> WorkbenchCaptureResult? {
        let context = await contextBuilder.build(
            enabledModuleIDs: enabledModuleIDs,
            pinnedModuleIDs: pinnedModuleIDs,
            clipboardPreview: clipboardPreview,
            selectionText: selectionText
        )
        guard let result = await captureService.capture(source: source, target: target, context: context) else {
            return nil
        }
        var resolvedAttribution = attribution
        if source == .projectContext, attribution.sourceKind != .command {
            resolvedAttribution = WorkbenchCaptureAttribution(
                sourceKind: .project,
                followUp: attribution.followUp
            )
        }
        await captureService.applyResult(result, context: context, attribution: resolvedAttribution)
        await MainActor.run {
            captureService.stagePendingState(for: result)
        }
        return result
    }
}
