import Foundation
import LumaCore
import LumaModules
import Testing

@Test func projectIndexMatchByLabelUsesNameAliasAndBasename() {
    let records = [
        ProjectRecord(name: "Luma", path: "/Users/dev/Luma", aliases: ["launcher"]),
        ProjectRecord(name: "Other", path: "/tmp/Other", aliases: [])
    ]
    let index = ProjectIndex(records: records)
    #expect(index.matchByLabel("Luma")?.path == "/Users/dev/Luma")
    #expect(index.matchByLabel("launcher")?.path == "/Users/dev/Luma")
    #expect(index.matchByLabel("Other")?.path == "/tmp/Other")
}

@Test func snippetVariableExpanderSupportsCoreVariables() {
    let context = SnippetExpansionContext(
        clipboardText: "clip",
        selectionText: "sel",
        projectName: "Luma",
        projectPath: "/Users/dev/Luma",
        filename: "App.swift",
        now: Date(timeIntervalSince1970: 1_700_000_000)
    )
    let output = SnippetVariableExpander.expand(
        "p={{project}} path={{project_path}} f={{file}} c={{clipboard}} s={{selection}}",
        context: context
    )
    #expect(output.contains("p=Luma"))
    #expect(output.contains("path=/Users/dev/Luma"))
    #expect(output.contains("f=App.swift"))
    #expect(output.contains("c=clip"))
    #expect(output.contains("s=sel"))
}

@Test func snippetVariableExpanderSupportsUUIDAndTimestamp() {
    let context = SnippetExpansionContext(now: Date(timeIntervalSince1970: 1_700_000_000))
    let output = SnippetVariableExpander.expand("id={{uuid}} ts={{timestamp}}", context: context)
    #expect(!output.contains("{{uuid}}"))
    #expect(!output.contains("{{timestamp}}"))
}

@Test func notesQueryParserCaptureToDaily() {
    #expect(NotesQueryParser.parse(payload: "cap buy milk", knownTemplates: []) == .captureToDaily(text: "buy milk"))
}
