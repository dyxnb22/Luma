import Foundation
import LumaCore
import Testing

@Test func scriptRunnerSecurityRejectsArbitraryPath() {
    #expect(throws: ScriptRunnerSecurityPolicy.ValidationError.self) {
        try ScriptRunnerSecurityPolicy.validateExecutable("/usr/bin/id")
    }
}

@Test func scriptRunnerSecurityAllowsLumaCommandsDir() throws {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let script = home.appendingPathComponent(".luma/commands/test.sh")
    try FileManager.default.createDirectory(
        at: script.deletingLastPathComponent(),
        withIntermediateDirectories: true
    )
    FileManager.default.createFile(atPath: script.path, contents: Data())
    defer { try? FileManager.default.removeItem(at: script) }
    try ScriptRunnerSecurityPolicy.validateExecutable(script.path)
}

@Test func scriptRunnerSecuritySanitizedEnvironmentOmitsSecrets() {
    var env = ProcessInfo.processInfo.environment
    env["API_KEY"] = "super-secret"
    let sanitized = ScriptRunnerSecurityPolicy.sanitizedEnvironment(from: env)
    #expect(sanitized["API_KEY"] == nil)
    #expect(sanitized["HOME"] != nil || sanitized["PATH"] != nil)
}

@Test func scriptRunnerSecurityRedactedMetadataOmitsFullPath() {
    let line = ScriptRunnerSecurityPolicy.redactedRunMetadata(
        executable: "/Users/me/.luma/commands/deploy.sh",
        exitCode: 0
    )
    #expect(line.contains("deploy.sh"))
    #expect(!line.contains("/Users/me"))
}
