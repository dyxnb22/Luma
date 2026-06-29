import AppKit
import LumaCore
import LumaModules

enum NotesCaptureHelper {
    @MainActor
    static func appendToDailyNote(_ text: String, openAfterCapture: Bool = true) async -> NotesDailyCaptureOutcome {
        guard let env = LauncherEnvironment.current else { return .failed }
        let outcome = await env.notesModule.captureTextToDailyNote(text)
        switch outcome {
        case .appended(let url):
            await HomeSuggestionMemory.shared.recordDailyNoteOpened()
            env.showStatus(LauncherStatusMessages.savedToDailyNote)
            if openAfterCapture {
                env.onHideLauncher()
                NSWorkspace.shared.open(url)
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
