import Foundation
import LumaCore
import Testing

@Suite(.serialized)
struct DiagnosticsExportRedactionTests {
    @Test func diagnosticsExportRedactsSensitiveBreadcrumbFields() {
        let input = "module=secrets query=super-secret clipboard=paste secret=abc noteBody=hello token=xyz password=pw apiKey=key payload={\"x\":1} status=timeout"
        let redacted = DiagnosticsExport.redactBreadcrumb(input)
        #expect(!redacted.contains("super-secret"))
        #expect(!redacted.contains("paste"))
        #expect(!redacted.contains("abc"))
        #expect(!redacted.contains("hello"))
        #expect(!redacted.contains("xyz"))
        #expect(!redacted.contains("pw"))
        #expect(!redacted.contains("\"x\":1"))
        #expect(redacted.contains("module=secrets"))
        #expect(redacted.contains("status=timeout"))
    }

    @Test func diagnosticsPayloadBuildsWithoutSensitiveKeys() {
        LauncherPerfCounters.reset()
        LauncherDurationRecorder.reset()
        LauncherPerfCounters.increment(.panelHide)
        let payload = DiagnosticsExport.buildPayload(
            appVersion: "1.0",
            buildNumber: "1",
            latencyP95: 12.5,
            breadcrumbs: ["module=todo status=timeout duration=40"]
        )
        #expect(payload.perfCounters["panel.hide"] == 1)
        #expect(payload.appVersion == "1.0")
        #expect(payload.breadcrumbs.first?.contains("module=todo") == true)
    }

    @Test func durationRecorderThreadSafeSummary() {
        LauncherDurationRecorder.reset()
        LauncherDurationRecorder.record(category: .moduleHandle, key: "apps", milliseconds: 5)
        LauncherDurationRecorder.record(category: .moduleHandle, key: "apps", milliseconds: 15)
        let summary = LauncherDurationRecorder.exportSummary()
        #expect(summary["module.handle.apps.count"] == 2)
        #expect(summary["module.handle.apps.p95"] != nil)
    }
}
