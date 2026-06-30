import Foundation
import LumaCore

/// Pure capture logic shared by the app capture service and tests.
public enum WorkbenchCaptureEngine {
    public static func capture(
        source: WorkbenchCaptureSource,
        target: WorkbenchCaptureTarget,
        context: WorkbenchContext
    ) -> WorkbenchCaptureResult? {
        guard context.isEnabled(target.moduleID) else { return nil }

        switch target {
        case .snippetDraft:
            return captureSnippet(source: source, context: context)
        case .quicklinkDraft:
            return captureQuicklink(source: source, context: context)
        case .todoDraft:
            return captureTodo(source: source, context: context)
        case .noteDraft:
            return captureNote(source: source, context: context)
        case .projectSnippetDraft:
            return captureProjectSnippet(context: context)
        }
    }

    private static func captureSnippet(
        source: WorkbenchCaptureSource,
        context: WorkbenchContext
    ) -> WorkbenchCaptureResult? {
        guard let text = WorkbenchCaptureDraftBuilder.captureText(source: source, context: context) else {
            return nil
        }
        let draft: SnippetDraft
        if source == .projectContext, let project = context.currentProject {
            draft = WorkbenchCaptureDraftBuilder.buildProjectLinkedSnippet(context: project, text: text)
        } else {
            draft = WorkbenchCaptureDraftBuilder.buildSnippetDraft(text: text)
        }
        let payload = WorkbenchCaptureDraftBuilder.buildSnippetPayload(draft)
        let json = try? JSONEncoder().encode(draft)
        return WorkbenchCaptureResult(
            target: .snippetDraft,
            moduleID: .snippets,
            preview: draft.trigger,
            openDetailPayload: payload,
            resumeDraftJSON: json
        )
    }

    private static func captureQuicklink(
        source: WorkbenchCaptureSource,
        context: WorkbenchContext
    ) -> WorkbenchCaptureResult? {
        if source == .projectContext, let project = context.currentProject,
           let draft = ProjectContextSuggestions.quicklinkDraft(for: project) {
            let payload = WorkbenchCaptureDraftBuilder.buildQuicklinkPayload(draft)
            let json = try? JSONEncoder().encode(draft)
            return WorkbenchCaptureResult(
                target: .quicklinkDraft,
                moduleID: .quicklinks,
                preview: draft.trigger,
                openDetailPayload: payload,
                resumeDraftJSON: json
            )
        }
        guard let text = WorkbenchCaptureDraftBuilder.captureText(source: source, context: context),
              let draft = WorkbenchCaptureDraftBuilder.buildQuicklinkDraft(text: text) else {
            return nil
        }
        let payload = WorkbenchCaptureDraftBuilder.buildQuicklinkPayload(draft)
        let json = try? JSONEncoder().encode(draft)
        return WorkbenchCaptureResult(
            target: .quicklinkDraft,
            moduleID: .quicklinks,
            preview: draft.trigger,
            openDetailPayload: payload,
            resumeDraftJSON: json
        )
    }

    private static func captureTodo(
        source: WorkbenchCaptureSource,
        context: WorkbenchContext
    ) -> WorkbenchCaptureResult? {
        guard let text = WorkbenchCaptureDraftBuilder.captureText(source: source, context: context) else {
            return nil
        }
        let captureText: String
        if source == .projectContext, let project = context.currentProject {
            captureText = ProjectContextSuggestions.todoDraft(for: project, text: text)
        } else {
            captureText = WorkbenchCaptureDraftBuilder.buildTodoCaptureText(text)
        }
        return WorkbenchCaptureResult(
            target: .todoDraft,
            moduleID: .todo,
            preview: captureText,
            openDetailPayload: nil,
            resumeDraftJSON: nil
        )
    }

    private static func captureNote(
        source: WorkbenchCaptureSource,
        context: WorkbenchContext
    ) -> WorkbenchCaptureResult? {
        guard let text = WorkbenchCaptureDraftBuilder.captureText(source: source, context: context) else {
            return nil
        }
        let captureText: String
        if source == .projectContext, let project = context.currentProject {
            captureText = ProjectContextSuggestions.noteCaptureText(for: project, text: text)
        } else {
            captureText = text
        }
        let payload = WorkbenchCaptureDraftBuilder.buildNoteCapturePayload(text: captureText)
        return WorkbenchCaptureResult(
            target: .noteDraft,
            moduleID: .notes,
            preview: String(captureText.prefix(48)),
            actionPayload: payload,
            openDetailPayload: nil,
            resumeDraftJSON: nil
        )
    }

    private static func captureProjectSnippet(context: WorkbenchContext) -> WorkbenchCaptureResult? {
        guard context.currentProject != nil,
              let snippet = captureSnippet(source: .projectContext, context: context) else { return nil }
        return WorkbenchCaptureResult(
            target: .projectSnippetDraft,
            moduleID: snippet.moduleID,
            preview: snippet.preview,
            actionPayload: snippet.actionPayload,
            openDetailPayload: snippet.openDetailPayload,
            resumeDraftJSON: snippet.resumeDraftJSON
        )
    }
}
