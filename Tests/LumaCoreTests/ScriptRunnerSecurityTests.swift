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

@Test func scriptRunnerSecurityRejectsSymlinkToSystemBinary() throws {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let commandsDir = home.appendingPathComponent(".luma/commands", isDirectory: true)
    try FileManager.default.createDirectory(at: commandsDir, withIntermediateDirectories: true)
    let link = commandsDir.appendingPathComponent("escape.sh")
    try? FileManager.default.removeItem(at: link)
    try FileManager.default.createSymbolicLink(
        at: link,
        withDestinationURL: URL(fileURLWithPath: "/usr/bin/id")
    )
    defer { try? FileManager.default.removeItem(at: link) }
    #expect(throws: ScriptRunnerSecurityPolicy.ValidationError.self) {
        try ScriptRunnerSecurityPolicy.validateExecutable(link.path)
    }
}

@Test func scriptRunnerSecurityRejectsSensitiveCWD() {
    let home = FileManager.default.homeDirectoryForCurrentUser.path
    #expect(throws: ScriptRunnerSecurityPolicy.ValidationError.self) {
        try ScriptRunnerSecurityPolicy.validateWorkingDirectory("\(home)/.ssh")
    }
}

@Test func scriptRunnerSecurityAllowsCommandsCWD() throws {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let commandsDir = home.appendingPathComponent(".luma/commands", isDirectory: true)
    try FileManager.default.createDirectory(at: commandsDir, withIntermediateDirectories: true)
    try ScriptRunnerSecurityPolicy.validateWorkingDirectory(commandsDir.path)
}

@Test func scriptRunnerSecurityAllowsScriptInsideSymlinkedCommandsDir() throws {
    let home = FileManager.default.homeDirectoryForCurrentUser
    let realDir = home.appendingPathComponent(".luma/commands-real-\(UUID().uuidString)", isDirectory: true)
    let linkDir = home.appendingPathComponent(".luma/commands-link-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: realDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: realDir) }
    defer { try? FileManager.default.removeItem(at: linkDir) }
    try FileManager.default.createSymbolicLink(at: linkDir, withDestinationURL: realDir)
    let script = realDir.appendingPathComponent("deploy.sh")
    FileManager.default.createFile(atPath: script.path, contents: Data())
    defer { try? FileManager.default.removeItem(at: script) }
    try ScriptRunnerSecurityPolicy.validateExecutable(
        script.path,
        allowedDirectories: [linkDir]
    )
}
