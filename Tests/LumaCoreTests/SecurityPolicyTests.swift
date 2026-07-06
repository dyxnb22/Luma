import Foundation
import LumaCore
import Testing

@Test func pathContainmentRejectsOutsideRoot() {
    let root = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("notes-root")
    #expect(throws: PathContainmentError.pathOutsideRoot) {
        try PathContainment.validateContained(path: "/etc/passwd", in: root)
    }
}

@Test func pathContainmentAcceptsInsideRoot() throws {
    let root = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("notes-root")
    let inside = root.appendingPathComponent("Inbox/note.md").path
    try PathContainment.validateContained(path: inside, in: root)
}

@Test func externalURLPolicyBlocksJavascript() {
    let url = URL(string: "javascript:alert(1)")!
    #expect(throws: ExternalURLPolicyError.self) {
        try ExternalURLPolicy.validateOpenURL(url)
    }
}

@Test func externalURLPolicyAllowsHTTPS() throws {
    let url = URL(string: "https://example.com")!
    try ExternalURLPolicy.validateOpenURL(url)
}

@Test func externalURLPolicyAllowsSystemPreferencesScheme() throws {
    let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders")!
    try ExternalURLPolicy.validateOpenURL(url)
}

@Test func externalURLPolicyBlocksFileByDefault() {
    let url = URL(fileURLWithPath: "/etc/passwd")
    #expect(throws: ExternalURLPolicyError.self) {
        try ExternalURLPolicy.validateOpenURL(url)
    }
}
