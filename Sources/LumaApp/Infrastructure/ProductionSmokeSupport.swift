import AppKit
import Foundation
import LumaCore

/// Shared helpers for signed-app `LUMA_QA_*` production smokes.
enum ProductionSmokeSupport {
    static var shouldAutoExit: Bool {
        ProcessInfo.processInfo.environment["LUMA_QA_AUTO_EXIT"] == "1"
    }

    static func finish(artifact: String) {
        CrashLogRecording.record("qa.smoke.completed artifact=\(artifact)")
        guard shouldAutoExit else { return }
        DispatchQueue.main.async {
            NSApp.terminate(nil)
        }
    }
}
