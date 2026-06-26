import AppKit
import LumaCore
import LumaModules

@MainActor
final class CurrentProjectDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Current Project"
    let detailView: NSView
    let usesSharedTopBar = true

    private let onRunProjectAction: (ProjectAction, @escaping () -> Void) -> Void
    private let stackView = NSStackView()
    private var pendingActions: [ProjectAction] = []
    private var context: CurrentProjectContext?

    init(onRunProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void) {
        self.onRunProjectAction = onRunProjectAction
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        context = LauncherSharedState.pendingCurrentProjectContext
        LauncherSharedState.pendingCurrentProjectContext = nil
        rebuildActions()
    }

    func deactivate() {
        pendingActions.removeAll()
    }

    private func setup(chrome: BaseDetailContainer) {
        stackView.orientation = .vertical
        stackView.alignment = .leading
        stackView.spacing = 8
        stackView.translatesAutoresizingMaskIntoConstraints = false
        chrome.setContent(stackView, embedInScroll: false)
    }

    private func rebuildActions() {
        stackView.arrangedSubviews.forEach { $0.removeFromSuperview() }
        pendingActions.removeAll()

        guard let context else {
            addLabel("No IDE project detected.")
            return
        }

        addLabel("In \(context.frontAppName): \(context.projectLabel)")
        if let path = context.matchedProjectPath {
            addLabel(displayPath(path))
        }

        guard let path = context.matchedProjectPath else {
            addLabel("Project path not matched in projects.json.")
            return
        }

        let name = context.projectName ?? context.projectLabel
        addActionButton("Open Terminal here", action: .openTerminal(path: path))
        addActionButton("Open Finder", action: .reveal(path))
        addActionButton("Open Notes for project", action: .openNotes(path: path, projectName: name))
        addActionButton("Copy path", action: .copyPath(path))
    }

    private func addLabel(_ text: String) {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: 13)
        label.textColor = .secondaryLabelColor
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

    @objc private func runAction(_ sender: NSButton) {
        guard pendingActions.indices.contains(sender.tag) else { return }
        onRunProjectAction(pendingActions[sender.tag]) {}
    }

    private func displayPath(_ path: String) -> String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if path.hasPrefix(home) { return "~" + path.dropFirst(home.count) }
        return path
    }
}
