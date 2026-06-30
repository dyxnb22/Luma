import Foundation

/// Per-activity draft snapshot for resuming a specific capture (not the global latest draft).
public enum WorkbenchActivityResumePayload: Sendable, Equatable, Codable {
    case snippetDraft(Data)
    case quicklinkDraft(Data)
    case todoCapture(String)

    public static func from(result: WorkbenchCaptureResult) -> WorkbenchActivityResumePayload? {
        switch result.target {
        case .snippetDraft, .projectSnippetDraft:
            guard let data = result.resumeDraftJSON else { return nil }
            return .snippetDraft(data)
        case .quicklinkDraft:
            guard let data = result.resumeDraftJSON else { return nil }
            return .quicklinkDraft(data)
        case .todoDraft:
            let text = result.preview.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return nil }
            return .todoCapture(text)
        case .noteDraft:
            return nil
        }
    }

    public func encoded() -> Data? {
        try? JSONEncoder().encode(self)
    }

    public static func decode(from data: Data) -> WorkbenchActivityResumePayload? {
        try? JSONDecoder().decode(WorkbenchActivityResumePayload.self, from: data)
    }
}

public extension WorkbenchActivityEntry {
    var resumablePayload: WorkbenchActivityResumePayload? {
        guard let resumePayloadJSON else { return nil }
        return WorkbenchActivityResumePayload.decode(from: resumePayloadJSON)
    }

    var isResumableDraft: Bool {
        resumablePayload != nil
    }
}
