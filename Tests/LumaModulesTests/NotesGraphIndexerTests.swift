import Testing
@testable import LumaModules

@Test func notesGraphExtractsWikiLinksAndTags() {
    let graph = NotesGraphIndexer.index(files: [
        "/vault/Inbox.md": "See [[System Design]] #inbox #learning",
        "/vault/System Design.md": "Latency notes #architecture"
    ])

    #expect(graph.nodes.count == 2)
    #expect(graph.edges.contains(NoteEdge(from: "/vault/Inbox.md", to: "System Design", kind: "wiki")))
    #expect(graph.edges.contains(NoteEdge(from: "/vault/Inbox.md", to: "inbox", kind: "tag")))
}

@Test func notesGraphHandlesEmptyFiles() {
    let graph = NotesGraphIndexer.index(files: ["/vault/Empty.md": ""])
    #expect(graph.nodes.first?.title == "Empty")
    #expect(graph.edges.isEmpty)
}

@Test func notesGraphDeduplicatesTagsPerNode() {
    let graph = NotesGraphIndexer.index(files: [
        "/vault/A.md": "#swift #swift [[B]]"
    ])
    #expect(graph.nodes.first?.tags == ["swift"])
    #expect(graph.edges.filter { $0.kind == "tag" }.count == 1)
}

@Test func notesGraphSupportsMultipleWikiLinks() {
    let graph = NotesGraphIndexer.index(files: [
        "/vault/A.md": "[[B]] [[C]] #map"
    ])
    #expect(graph.edges.contains(NoteEdge(from: "/vault/A.md", to: "B", kind: "wiki")))
    #expect(graph.edges.contains(NoteEdge(from: "/vault/A.md", to: "C", kind: "wiki")))
}
