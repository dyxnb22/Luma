import Foundation
import LumaCore
import LumaServices

public actor QuicklinksModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .quicklinks,
        displayName: "Quicklinks",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 5,
        queryTimeout: .milliseconds(15)
    )

    private let store: QuicklinksStore
    private var index = QuicklinksIndex(quicklinks: [])
    private var cachedQuicklinks: [Quicklink] = []

    public init(store: QuicklinksStore = QuicklinksStore()) {
        self.store = store
    }

    public func warmup(_ context: ModuleContext) async {
        await refreshCache()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if let payload = query.command?.payload ?? Self.extractManagePayload(raw: query.raw) {
            if ModuleHelp.isHelpQuery(payload) {
                return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
            }
            return ModuleResult(items: [manageRow()])
        }

        guard let match = index.match(raw: query.raw),
              let expansion = await expand(match.quicklink, query: match.query) else {
            return ModuleResult(items: [])
        }
        return ModuleResult(items: [row(for: expansion)])
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(QuicklinksAction.self, from: payload)
        switch decoded {
        case .open(let urlString, let bundleID):
            guard let url = URL(string: urlString) else { throw ModuleError.dataUnavailable }
            if let bundleID {
                await context.platform.workspace.openApplication(bundleID: bundleID, arguments: [url.absoluteString])
            } else {
                await context.platform.workspace.openURL(url)
            }
        case .copy(let url):
            await context.platform.pasteboard.write(url)
        case .revealConfig:
            await context.platform.workspace.revealInFinder(await store.configFileURL())
        case .prepareDraft:
            break
        }
    }

    public func allQuicklinks() async -> [Quicklink] {
        await store.all()
    }

    public func configFileURL() async -> URL {
        await store.configFileURL()
    }

    public func add(_ quicklink: Quicklink) async throws -> Quicklink {
        let saved = try await store.add(quicklink)
        await refreshCache()
        return saved
    }

    public func update(_ quicklink: Quicklink) async throws -> Quicklink {
        let saved = try await store.update(quicklink)
        await refreshCache()
        return saved
    }

    public func delete(id: UUID) async throws {
        try await store.delete(id: id)
        await refreshCache()
    }

    public func conflictingQuicklink(trigger: String, excluding id: UUID? = nil) async -> Quicklink? {
        await store.conflictingQuicklink(trigger: trigger, excluding: id)
    }

    public func duplicateQuicklink(urlTemplate: String, excluding id: UUID? = nil) async -> Quicklink? {
        await store.duplicateQuicklink(urlTemplate: urlTemplate, excluding: id)
    }

    public func validateURLTemplate(_ template: String) -> String? {
        QuicklinksStore.validateURLTemplate(template)
    }

    public func sampleExpansion(
        template: String,
        query: String = "swift package",
        clipboard: String? = nil,
        selection: String? = nil,
        project: String? = nil,
        projectPath: String? = nil
    ) -> String {
        QuicklinkTemplateRenderer.render(
            template: template,
            query: query,
            clipboard: clipboard,
            selection: selection,
            project: project,
            projectPath: projectPath
        )
    }

    private func refreshCache() async {
        cachedQuicklinks = await store.all()
        index = QuicklinksIndex(quicklinks: cachedQuicklinks)
    }

    private func expand(_ quicklink: Quicklink, query: String) async -> QuicklinkExpansion? {
        let project = await CurrentProjectService.shared.snapshot()
        let selection = await SelectionSnapshotService.shared.snapshot()
        let clipboard = await PasteboardService().readString()
        let urlString = QuicklinkTemplateRenderer.render(
            template: quicklink.urlTemplate,
            query: query,
            clipboard: clipboard,
            selection: selection,
            project: project?.projectName ?? project?.projectLabel,
            projectPath: project?.matchedProjectPath
        )
        guard let url = URL(string: urlString), let scheme = url.scheme, !scheme.isEmpty else { return nil }
        return QuicklinkExpansion(quicklink: quicklink, query: query, urlString: urlString, url: url)
    }

    private func manageRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "manage"),
            title: "Manage Quicklinks",
            titleAttributed: AttributedString("Manage Quicklinks"),
            subtitle: "Add, edit, or remove URL templates",
            icon: .symbol("link"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "manage"),
                title: "Open Quicklinks",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
            rowKind: .starter
        )
    }

    private func row(for expansion: QuicklinkExpansion) -> ResultItem {
        let q = expansion.quicklink
        let openPayload = (try? ModuleActionCoding.encode(QuicklinksAction.open(url: expansion.urlString, bundleID: q.openWith))) ?? Data()
        let copyPayload = (try? ModuleActionCoding.encode(QuicklinksAction.copy(url: expansion.urlString))) ?? Data()
        let revealPayload = (try? ModuleActionCoding.encode(QuicklinksAction.revealConfig)) ?? Data()
        let openAction: Action
        if q.openWith == nil {
            openAction = Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(q.id.uuidString)"),
                title: "Open \(q.name)",
                kind: .openURL(expansion.url)
            )
        } else {
            openAction = Action(
                id: ActionID(module: Self.manifest.identifier, key: "open.\(q.id.uuidString)"),
                title: "Open \(q.name)",
                kind: .custom(payload: openPayload, handler: Self.manifest.identifier)
            )
        }
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: q.id.uuidString),
            title: q.name,
            titleAttributed: AttributedString(q.name),
            subtitle: expansion.urlString,
            icon: .symbol(q.icon ?? "link"),
            primaryAction: openAction,
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "copy.\(q.id.uuidString)"),
                    title: "Copy URL",
                    kind: .custom(payload: copyPayload, handler: Self.manifest.identifier)
                ),
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "config"),
                    title: "Reveal Quicklinks Config",
                    kind: .custom(payload: revealPayload, handler: Self.manifest.identifier)
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    public static func extractManagePayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "quicklinks" || lower == "ql" { return "" }
        if lower.hasPrefix("quicklinks ") {
            return String(trimmed.dropFirst("quicklinks ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("ql ") {
            return String(trimmed.dropFirst("ql ".count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
