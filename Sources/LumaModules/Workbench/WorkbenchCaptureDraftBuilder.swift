import Foundation
import LumaCore

/// Pure draft builders for workbench capture flows.
public enum WorkbenchCaptureDraftBuilder {
    public static func captureText(
        source: WorkbenchCaptureSource,
        context: WorkbenchContext
    ) -> String? {
        switch source {
        case .selection:
            return nonEmpty(context.selectionText)
        case .clipboardText:
            guard let preview = context.clipboardPreview,
                  ClipboardTextOps.classify(preview) != .url else { return nil }
            return nonEmpty(preview)
        case .clipboardURL:
            if let url = context.clipboardURL {
                return url.absoluteString
            }
            guard let preview = context.clipboardPreview,
                  let url = URLTextParser.firstHTTPURL(in: preview) else { return nil }
            return url.absoluteString
        case .projectContext:
            return context.currentProject?.projectName ?? context.currentProject?.projectLabel
        }
    }

    public static func buildSnippetDraft(text: String) -> SnippetDraft {
        SnippetDraft.fromClipboard(text)
    }

    public static func buildQuicklinkDraft(text: String) -> URLQuicklinkDraft? {
        TextQuicklinkDraftSource(text: text).quicklinkDraft()
    }

    public static func buildTodoCaptureText(_ text: String) -> String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public static func buildNoteCapturePayload(text: String) -> Data? {
        try? ModuleActionCoding.encode(NotesAction.captureToDaily(text: text))
    }

    public static func buildSnippetPayload(_ draft: SnippetDraft) -> Data? {
        try? ModuleActionCoding.encode(SnippetsAction.prepareDraft(draft))
    }

    public static func buildQuicklinkPayload(_ draft: URLQuicklinkDraft) -> Data? {
        try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))
    }

    public static func buildProjectLinkedSnippet(
        context: CurrentProjectContext,
        text: String
    ) -> SnippetDraft {
        var draft = ProjectContextSuggestions.snippetDraft(for: context)
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty {
            draft = SnippetDraft(
                title: draft.title,
                trigger: draft.trigger,
                content: trimmed,
                tags: draft.tags
            )
        }
        return draft
    }

    private static func nonEmpty(_ text: String?) -> String? {
        guard let text else { return nil }
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
