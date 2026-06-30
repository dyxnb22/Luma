import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

@MainActor
final class CurrentProjectDetailView: NSObject, ModuleDetailView {
  let moduleTitle = "Current Project"
  let detailView: NSView
  let usesSharedTopBar = true

  private let config: ConfigurationStore
  private let onRunProjectAction: (ProjectAction, @escaping () -> Void) -> Void
  private let onRunWorkbenchCapture: (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void
  private let onRunWorkspaceRow: (CurrentProjectWorkspaceRowAction) -> Void
  private let stackView = NSStackView()
  private var pendingActions: [ProjectAction] = []
  private var context: CurrentProjectContext?
  private var loadGeneration = 0
  private var loadTask: Task<Void, Never>?

  init(
    config: ConfigurationStore,
    onRunProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void,
    onRunWorkbenchCapture: @escaping (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void,
    onRunWorkspaceRow: @escaping (CurrentProjectWorkspaceRowAction) -> Void
  ) {
    self.config = config
    self.onRunProjectAction = onRunProjectAction
    self.onRunWorkbenchCapture = onRunWorkbenchCapture
    self.onRunWorkspaceRow = onRunWorkspaceRow
    let chrome = BaseDetailContainer()
    self.detailView = chrome
    super.init()
    setup(chrome: chrome)
  }

  func activate() {
    context = LauncherSharedState.pendingCurrentProjectContext
    LauncherSharedState.pendingCurrentProjectContext = nil
    loadTask?.cancel()
    loadGeneration &+= 1
    let generation = loadGeneration
    render(model: CurrentProjectWorkspaceModelBuilder.loading(context: context), context: context)
    loadTask = Task { await loadAndRender(generation: generation) }
  }

  func deactivate() {
    loadTask?.cancel()
    loadTask = nil
    pendingActions.removeAll()
  }

  private func setup(chrome: BaseDetailContainer) {
    stackView.orientation = .vertical
    stackView.alignment = .leading
    stackView.spacing = 8
    stackView.translatesAutoresizingMaskIntoConstraints = false
    chrome.setContent(stackView, embedInScroll: true)
  }

  private func loadAndRender(generation: Int) async {
    let projectContext = context
    let projectIdentity = projectContext.map(WorkbenchProjectIdentity.init(context:))
    async let enabledModules = config.enabledModules()
      ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
    async let allEntries = WorkbenchActivityStore.shared.allEntries()
    async let activitySnapshot = WorkbenchActivityStore.shared.activitySnapshot(
      projectIdentity: projectIdentity
    )
    let entries = await allEntries
    await WorkbenchLinkStore.shared.backfillFromActivitiesIfEmpty(entries)
    async let linkSnapshot = WorkbenchLinkStore.shared.snapshot(
      for: projectIdentity?.identity,
      limit: 10
    )
    let enabled = await enabledModules
    let snapshot = await activitySnapshot
    let links = await linkSnapshot
    guard !Task.isCancelled, generation == loadGeneration else { return }

    let notePath: String?
    if let projectContext, let path = projectContext.matchedProjectPath {
      let name = projectContext.projectName ?? projectContext.projectLabel
      notePath = ProjectNotesPaths.existingNotePath(projectPath: path, projectName: name)
    } else {
      notePath = nil
    }

    let model = CurrentProjectWorkspaceModelBuilder.build(
      context: projectContext,
      activitySnapshot: snapshot,
      linkSnapshot: WorkbenchLinkSnapshot(currentProjectLinks: links),
      enabledModuleIDs: enabled,
      existingProjectNotePath: notePath
    )
    guard !Task.isCancelled, generation == loadGeneration else { return }
    render(model: model, context: projectContext)
  }

  private func render(model: CurrentProjectWorkspaceModel, context: CurrentProjectContext?) {
    stackView.arrangedSubviews.forEach { $0.removeFromSuperview() }
    pendingActions.removeAll()

    addHeading(model.headerTitle)
    for line in model.headerLines {
      addLabel(line)
    }

    if !model.quickCaptureActions.isEmpty || model.quickCaptureDisabledHint != nil {
      addHeading("Quick capture")
      for action in model.quickCaptureActions {
        addCaptureButton(action.title, source: action.source, target: action.target)
      }
      if let hint = model.quickCaptureDisabledHint {
        addLabel(hint)
      }
    }

    if !model.linkedItemRows.isEmpty {
      addHeading("Linked items")
      for row in model.linkedItemRows {
        let suffix = row.subtitle.isEmpty ? "" : " — \(row.subtitle)"
        addWorkspaceRowButton("\(row.title)\(suffix)", action: row.action)
      }
    }

    if !model.recentActivityRows.isEmpty {
      addHeading("Recent project activity")
      for row in model.recentActivityRows {
        let suffix = row.subtitle.isEmpty ? "" : " — \(row.subtitle)"
        if row.isInteractive {
          addWorkspaceRowButton("\(row.title)\(suffix)", action: row.action)
        } else {
          addLabel("• \(row.title)\(suffix)")
        }
      }
    }

    guard model.showsProjectActions, let context, let path = context.matchedProjectPath else { return }
    let name = context.projectName ?? context.projectLabel
    addHeading("Project actions")
    addActionButton("Open Terminal here", action: .openTerminal(path: path))
    addActionButton("Open Finder", action: .reveal(path))
    addActionButton(CrossModuleActionTitles.openNotesForProject, action: .openNotes(path: path, projectName: name))
    addActionButton("Copy path", action: .copyPath(path))
  }

  private func addHeading(_ text: String) {
    let label = NSTextField(labelWithString: text)
    label.font = .systemFont(ofSize: 15, weight: .semibold)
    stackView.addArrangedSubview(label)
  }

  private func addLabel(_ text: String) {
    let label = NSTextField(labelWithString: text)
    label.font = .systemFont(ofSize: 13)
    label.textColor = .secondaryLabelColor
    label.lineBreakMode = .byWordWrapping
    label.maximumNumberOfLines = 0
    label.preferredMaxLayoutWidth = 420
    stackView.addArrangedSubview(label)
  }

  private func addActionButton(_ title: String, action: ProjectAction) {
    let index = pendingActions.count
    pendingActions.append(action)
    let button = NSButton(title: title, target: self, action: #selector(runAction(_:)))
    button.bezelStyle = .rounded
    button.tag = index
    stackView.addArrangedSubview(button)
  }

  private func addCaptureButton(
    _ title: String,
    source: WorkbenchCaptureSource,
    target: WorkbenchCaptureTarget
  ) {
    let button = NSButton(title: title, target: self, action: #selector(runCapture(_:)))
    button.bezelStyle = .rounded
    button.identifier = NSUserInterfaceItemIdentifier("capture|\(source.rawValue)|\(target.rawValue)")
    stackView.addArrangedSubview(button)
  }

  private func addWorkspaceRowButton(_ title: String, action: CurrentProjectWorkspaceRowAction) {
    let key = workspaceRowKey(for: action)
    let button = NSButton(title: title, target: self, action: #selector(runWorkspaceRow(_:)))
    button.bezelStyle = .rounded
    button.identifier = NSUserInterfaceItemIdentifier(key)
    stackView.addArrangedSubview(button)
  }

  private func workspaceRowKey(for action: CurrentProjectWorkspaceRowAction) -> String {
    switch action {
    case .resumeActivity(let entryID):
      return "row|resume|\(entryID.uuidString)"
    case .openLinked(let linkID):
      return "row|link|\(linkID.uuidString)"
    case .openModule(let moduleID):
      return "row|open|\(moduleID.rawValue)"
    case .replaceQuery(let query):
      return "row|query|\(query)"
    case .openNotePath(let path):
      return "row|note|\(path)"
    case .status(let message):
      return "row|status|\(message)"
    }
  }

  private func workspaceRowAction(from key: String) -> CurrentProjectWorkspaceRowAction? {
    let parts = key.split(separator: "|", omittingEmptySubsequences: false).map(String.init)
    guard parts.count >= 3, parts[0] == "row" else { return nil }
    switch parts[1] {
    case "resume":
      guard let id = UUID(uuidString: parts[2]) else { return nil }
      return .resumeActivity(entryID: id)
    case "link":
      guard let id = UUID(uuidString: parts[2]) else { return nil }
      return .openLinked(linkID: id)
    case "open":
      return .openModule(moduleID: ModuleIdentifier(rawValue: parts[2]))
    case "query":
      return .replaceQuery(parts.dropFirst(2).joined(separator: "|"))
    case "note":
      return .openNotePath(parts.dropFirst(2).joined(separator: "|"))
    case "status":
      return .status(parts.dropFirst(2).joined(separator: "|"))
    default:
      return nil
    }
  }

  @objc private func runCapture(_ sender: NSButton) {
    guard let id = sender.identifier?.rawValue else { return }
    let parts = id.split(separator: "|", maxSplits: 2).map(String.init)
    guard parts.count == 3, parts[0] == "capture",
          let source = WorkbenchCaptureSource(rawValue: parts[1]),
          let target = WorkbenchCaptureTarget(rawValue: parts[2]) else { return }
    onRunWorkbenchCapture(source, target)
  }

  @objc private func runWorkspaceRow(_ sender: NSButton) {
    guard let id = sender.identifier?.rawValue,
          let action = workspaceRowAction(from: id) else { return }
    onRunWorkspaceRow(action)
  }

  @objc private func runAction(_ sender: NSButton) {
    guard pendingActions.indices.contains(sender.tag) else { return }
    onRunProjectAction(pendingActions[sender.tag]) {}
  }
}
