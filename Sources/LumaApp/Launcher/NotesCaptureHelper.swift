import AppKit
import LumaCore
import LumaModules
import LumaServices

enum NotesCaptureHelper {
    @MainActor
    static func appendToDailyNote(
        _ text: String,
        openAfterCapture: Bool = true,
        workspace: any WorkspaceClient = WorkspaceService()
    ) async -> NotesDailyCaptureOutcome {
        guard let env = LauncherEnvironment.current else { return .failed }
        let outcome = await env.notesModule.captureTextToDailyNote(text)
        switch outcome {
        case .appended(let url):
            env.showStatus(LauncherStatusMessages.savedToDailyNote)
            if openAfterCapture {
                env.onHideLauncher()
                try? await workspace.openLocalFileURL(url)
            }
        case .rootNotConfigured:
            env.showStatus(LauncherStatusMessages.notesRootNotConfigured)
            env.openModuleDetail(.notes)
        case .failed:
            env.showStatus(LauncherStatusMessages.noteSaveFailed)
        }
        return outcome
    }
}
