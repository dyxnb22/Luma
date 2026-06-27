import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing

@Test func projectScannerFindsGitAndSwiftPackageProjects() throws {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-project-scan-\(UUID().uuidString)", isDirectory: true)
    defer { try? FileManager.default.removeItem(at: root) }
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

    let gitProject = root.appendingPathComponent("GitProject", isDirectory: true)
    try FileManager.default.createDirectory(at: gitProject, withIntermediateDirectories: true)
    try Data().write(to: gitProject.appendingPathComponent(".git"))

    let swiftProject = root.appendingPathComponent("SwiftPkg", isDirectory: true)
    try FileManager.default.createDirectory(at: swiftProject, withIntermediateDirectories: true)
    try "import PackageDescription".write(to: swiftProject.appendingPathComponent("Package.swift"), atomically: true, encoding: .utf8)

    let nested = root.appendingPathComponent("Org", isDirectory: true)
    try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
    let nestedProject = nested.appendingPathComponent("NestedApp", isDirectory: true)
    try FileManager.default.createDirectory(at: nestedProject, withIntermediateDirectories: true)
    try Data().write(to: nestedProject.appendingPathComponent("package.json"))

    let records = ProjectScanner.scan(roots: [root.path])
    let names = Set(records.map(\.name))
    #expect(names.contains("GitProject"))
    #expect(names.contains("SwiftPkg"))
    #expect(names.contains("NestedApp"))
}

@Test func projectIndexSearchesNameAliasAndPath() {
    let records = [
        ProjectRecord(name: "Luma", path: "/Users/dev/Luma", aliases: ["launcher"], preferredOpener: .cursor, pinned: true),
        ProjectRecord(name: "Notes", path: "/Users/dev/Notes", aliases: [], preferredOpener: .vscode)
    ]
    let index = ProjectIndex(records: records)

    #expect(index.search("luma").first?.record.name == "Luma")
    #expect(index.search("launcher").first?.record.name == "Luma")
    #expect(index.search("notes").first?.record.name == "Notes")
}

@Test func projectIndexHomeRecordsPreferPinnedAndRecent() {
    let records = [
        ProjectRecord(name: "Alpha", path: "/a", pinned: false),
        ProjectRecord(name: "Beta", path: "/b", pinned: true),
        ProjectRecord(name: "Gamma", path: "/c", pinned: false)
    ]
    let index = ProjectIndex(records: records)
    let home = index.homeRecords(limit: 2, recentPaths: ["/c"])
    #expect(home.map(\.name) == ["Beta", "Gamma"])
}

@Test func projectIndexNormalizesTildePaths() {
    let home = FileManager.default.homeDirectoryForCurrentUser.path
    let record = ProjectRecord(name: "Luma", path: "~/Developer/Luma", pinned: false)
    let index = ProjectIndex(records: [record])

    #expect(index.search("Luma").first?.record.path == "\(home)/Developer/Luma")
    #expect(index.homeRecords(recentPaths: ["~/Developer/Luma"]).first?.path == "\(home)/Developer/Luma")
}

@Test func projectsModuleReturnsRecentsForBareProj() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-projects-\(UUID().uuidString)", isDirectory: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

    let configURL = tempDir.appendingPathComponent("projects.json")
    let config = ProjectsConfig(
        roots: [],
        projects: [
            ProjectRecord(name: "Luma", path: "/Users/dev/Luma", aliases: ["luma"], preferredOpener: .cursor, pinned: true)
        ],
        recent: ["/Users/dev/Luma"]
    )
    let data = try JSONEncoder().encode(config)
    try data.write(to: configURL)

    let store = ProjectStore(fileURL: configURL)
    let module = ProjectsModule(store: store)
    await module.warmup(testModuleContext())

    let result = await module.handle(Query(raw: "proj", sequence: 0), context: QueryContext(deadline: .now))
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Luma")
}

@Test func projectsModuleSearchReturnsProject() async throws {
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-projects-search-\(UUID().uuidString)", isDirectory: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

    let configURL = tempDir.appendingPathComponent("projects.json")
    let config = ProjectsConfig(
        roots: [],
        projects: [
            ProjectRecord(name: "Luma", path: "/Users/dev/Luma", aliases: ["luma"], preferredOpener: .cursor)
        ],
        recent: []
    )
    try JSONEncoder().encode(config).write(to: configURL)

    let module = ProjectsModule(store: ProjectStore(fileURL: configURL))
    await module.warmup(testModuleContext())

    let result = await module.handle(Query(raw: "proj luma", sequence: 0), context: QueryContext(deadline: .now))
    #expect(result.items.count == 1)
    #expect(result.items.first?.title == "Luma")

    let secondaryTitles = Set(result.items.first?.secondaryActions.map(\.title) ?? [])
    #expect(secondaryTitles.contains("Open in Finder"))
    #expect(secondaryTitles.contains("Open in Terminal"))
    #expect(secondaryTitles.contains("Copy Path"))
    #expect(secondaryTitles.contains("Open Notes"))
    #expect(secondaryTitles.contains("Reveal Config"))
}

@Test func projectIndexSearchPerformanceStaysUnderBudget() {
    var records: [ProjectRecord] = []
    records.reserveCapacity(500)
    for index in 0..<500 {
        records.append(ProjectRecord(
            name: "Project \(index)",
            path: "/Users/dev/project-\(index)",
            aliases: ["p\(index)"]
        ))
    }
    let projectIndex = ProjectIndex(records: records)

    var samples: [Double] = []
    let clock = ContinuousClock()
    for query in ["proj", "project 42", "p 17", "dev"] {
        for _ in 0..<100 {
            let start = clock.now
            _ = projectIndex.search(query, limit: 8)
            let elapsed = start.duration(to: clock.now)
            samples.append(Double(elapsed.components.seconds) * 1000 + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000)
        }
    }

    let sorted = samples.sorted()
    let p95 = sorted[Int(Double(sorted.count - 1) * 0.95)]
    #expect(p95 < 20)
}

private func testModuleContext() -> ModuleContext {
    ModuleContext(
        logger: LumaLogger(),
        metrics: LumaMetrics(),
        database: ApplicationSupportPaths(),
        pasteboard: PasteboardService(),
        accessibility: AXService(),
        fileSystem: FSEventsService(),
        translation: TranslationService(),
        config: ConfigurationStore()
    )
}
