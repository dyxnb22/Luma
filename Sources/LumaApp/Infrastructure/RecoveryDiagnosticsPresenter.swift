import AppKit
import LumaCore

@MainActor
enum RecoveryDiagnosticsPresenter {
    static func showDoctorSummary(_ summary: LumaDiagnosticsSummary) {
        let alert = NSAlert()
        alert.messageText = summary.isHealthy ? "Luma Doctor" : "Luma Doctor — needs attention"
        let lines = summary.issues.map { issue in
            "[\(issue.severity.rawValue)] \(issue.message)"
        }
        alert.informativeText = lines.isEmpty
            ? "No blocking issues found."
            : lines.joined(separator: "\n")
        alert.alertStyle = summary.isHealthy ? .informational : .warning
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    static func showExportSuccess(url: URL, crashLogPath: String?) {
        let alert = NSAlert()
        alert.messageText = "Diagnostics exported"
        var details = "Saved to:\n\(url.path)"
        if let crashLogPath {
            details += "\n\nCrash log:\n\(crashLogPath)"
        }
        alert.informativeText = details
        alert.alertStyle = .informational
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    static func showExportFailure(_ error: Error) {
        let alert = NSAlert()
        alert.messageText = "Diagnostics export failed"
        alert.informativeText = error.localizedDescription
        alert.alertStyle = .warning
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }
}
