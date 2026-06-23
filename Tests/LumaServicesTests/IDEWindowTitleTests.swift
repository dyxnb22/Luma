import Testing
@testable import LumaServices

@Suite struct IDEWindowTitleTests {
    private let cursorBundle = "com.todesktop.230313mzl4w4u92"
    private let vscodeBundle = "com.microsoft.VSCode"
    private let ideaBundle = "com.jetbrains.intellij"

    @Test func cursorProjectOnlyTitle() {
        let label = IDEWindowTitle.sidebarLabel(rawTitle: "Luma", bundleID: cursorBundle, appName: "Cursor")
        #expect(label == "Luma")
    }

    @Test func cursorFileAndProjectTitle() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "main.java — Luma",
            bundleID: cursorBundle,
            appName: "Cursor"
        )
        #expect(label == "Luma")
    }

    @Test func cursorFileProjectAndAppTitle() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "README.md — LeetCode — Cursor",
            bundleID: cursorBundle,
            appName: "Cursor"
        )
        #expect(label == "LeetCode")
    }

    @Test func cursorProjectAndAppTitle() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "LeetCode — Cursor",
            bundleID: cursorBundle,
            appName: "Cursor"
        )
        #expect(label == "LeetCode")
    }

    @Test func vscodeUsesSameRules() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "App.tsx — my-app — Visual Studio Code",
            bundleID: vscodeBundle,
            appName: "Visual Studio Code"
        )
        #expect(label == "my-app")
    }

    @Test func intellijProjectFirst() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "LeetCode – Main.java – IntelliJ IDEA",
            bundleID: ideaBundle,
            appName: "IntelliJ IDEA"
        )
        #expect(label == "LeetCode")
    }

    @Test func nonIDEReturnsRawTitle() {
        let label = IDEWindowTitle.sidebarLabel(
            rawTitle: "Inbox — Google Chrome",
            bundleID: "com.google.Chrome",
            appName: "Google Chrome"
        )
        #expect(label == "Inbox — Google Chrome")
    }

    @Test func isIDEDetectsCursorAndVSCode() {
        #expect(IDEWindowTitle.isIDE(bundleID: cursorBundle))
        #expect(IDEWindowTitle.isIDE(bundleID: vscodeBundle))
        #expect(IDEWindowTitle.isIDE(bundleID: ideaBundle))
        #expect(!IDEWindowTitle.isIDE(bundleID: "com.google.Chrome"))
    }
}
