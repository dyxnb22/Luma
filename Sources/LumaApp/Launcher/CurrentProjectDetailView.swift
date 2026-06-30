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
  private let stackView = NSStackView()
  private var pendingActions: [ProjectAction] = []
  private var context: CurrentProjectContext?
  private var loadGeneration = 0
  private var loadTask: Task<Void, Never>?

  init(
    config: ConfigurationStore,
    onRunProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void,
    onRunWorkbenchCapture: @escaping (WorkbenchCaptureSource, WorkbenchCaptureTarget) -> Void
  ) {
    self.config = config
    self.onRunProjectAction = onRunProjectAction
    self.onRunWorkbenchCapture = onRunWorkbenchCapture
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
    async let activitySnapshot = WorkbenchActivityStore.shared.activitySnapshot(
      projectIdentity: projectIdentity
    )
    let enabled = await enabledModules
    let snapshot = await activitySnapshot
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

    if !model.recentActivityLines.isEmpty {
      addHeading("Recent project activity")
      for line in model.recentActivityLines {
        let suffix = line.subtitle.isEmpty ? "" : " — \(line.subtitle)"
        addLabel("• \(line.title)\(suffix)")
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
    button.identifier = NSUserInterfaceItemIdentifier("\(source.rawValue)|\(target.rawValue)")
    stackView.addArrangedSubview(button)
  }

  @objc private func runCapture(_ sender: NSButton) {
    guard let id = sender.identifier?.rawValue else { return }
    let parts = id.split(separator: "|", maxSplits: 1).map(String.init)
    guard parts.count == 2,
          let source = WorkbenchCaptureSource(rawValue: parts[0]),
          let target = WorkbenchCaptureTarget(rawValue: parts[1]) else { return }
    onRunWorkbenchCapture(source, target)
  }

  @objc private func runAction(_ sender: NSButton) {
    guard pendingActions.indices.contains(sender.tag) else { return }
    onRunProjectAction(pendingActions[sender.tag]) {}
  }
}
