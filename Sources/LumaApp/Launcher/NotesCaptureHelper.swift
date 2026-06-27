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
            if openAfterCapture {
                env.onHideLauncher()
                NSWorkspace.shared.open(url)
            }
        case .rootNotConfigured:
            env.openModuleDetail(.notes)
        case .failed:
            break
        }
        return outcome
    }
}
