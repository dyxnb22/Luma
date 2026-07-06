@preconcurrency import AppKit
import LumaModules

@MainActor
final class NotesImageToolsPanel: NSViewController {
    private let root: URL
    private let scanOrphansButton = NSButton(title: "Scan Orphans", target: nil, action: nil)
    private let scanBrokenButton = NSButton(title: "Scan Broken Links", target: nil, action: nil)
    private let migrateButton = NSButton(title: "Migrate Images to _assets/", target: nil, action: nil)
    private let typoraButton = NSButton(title: "Check Typora Config", target: nil, action: nil)
    private let closeButton = NSButton(title: "Close", target: nil, action: nil)
    private let resultsView = NSTextView()
    private var runningTask: Task<Void, Never>?

    init(root: URL) {
        self.root = root
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func loadView() {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 360))
        self.view = container

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false

        configureButton(scanOrphansButton, symbol: "photo.on.rectangle.angled")
        configureButton(scanBrokenButton, symbol: "link.badge.plus")
        configureButton(migrateButton, symbol: "folder.badge.gearshape")
        configureButton(typoraButton, symbol: "checkmark.seal")

        [scanOrphansButton, scanBrokenButton, migrateButton, typoraButton].forEach { button in
            button.bezelStyle = .rounded
            button.target = self
            stack.addArrangedSubview(button)
        }

        resultsView.isEditable = false
        resultsView.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        let scroll = NSScrollView()
        scroll.documentView = resultsView
        GeekUIKit.configureVerticalListScroll(scroll)
        scroll.translatesAutoresizingMaskIntoConstraints = false

        closeButton.bezelStyle = .rounded
        closeButton.target = self
        closeButton.action = #selector(closeSheet)
        closeButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(stack)
        container.addSubview(scroll)
        container.addSubview(closeButton)

        scanOrphansButton.action = #selector(scanOrphans)
        scanBrokenButton.action = #selector(scanBroken)
        migrateButton.action = #selector(migrate)
        typoraButton.action = #selector(checkTypora)

        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            stack.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),

            scroll.topAnchor.constraint(equalTo: stack.bottomAnchor, constant: 12),
            scroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            scroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            scroll.bottomAnchor.constraint(equalTo: closeButton.topAnchor, constant: -12),

            closeButton.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            closeButton.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -16)
        ])
    }

    private func configureButton(_ button: NSButton, symbol: String) {
        let config = NSImage.SymbolConfiguration(pointSize: 13, weight: .medium)
        button.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
            .withSymbolConfiguration(config)
        button.imagePosition = .imageLeading
        button.contentTintColor = .controlAccentColor
    }

    @objc private func closeSheet() {
        view.window?.sheetParent?.endSheet(view.window!)
    }

    private func setButtonsEnabled(_ enabled: Bool) {
        [scanOrphansButton, scanBrokenButton, migrateButton, typoraButton, closeButton].forEach { $0.isEnabled = enabled }
    }

    private func setResults(_ text: String) {
        resultsView.string = text
    }

    @objc private func scanOrphans() {
        runTool { tools in
            let report = await tools.scan()
            return "Orphans (\(report.orphans.count)):\n" + report.orphans.map(\.lastPathComponent).joined(separator: "\n")
        }
    }

    @objc private func scanBroken() {
        runTool { tools in
            let report = await tools.scan()
            let lines = report.brokenLinks.map { "\($0.0.lastPathComponent): \($0.1)" }
            return "Broken links (\(lines.count)):\n" + lines.joined(separator: "\n")
        }
    }

    @objc private func migrate() {
        runTool { [weak self] tools in
            let report = await tools.scan()
            let count = Set(report.brokenLinks.map(\.1) + report.orphans.map(\.lastPathComponent)).count
            guard await self?.confirmMigrate(count: count) == true else {
                return "Migration cancelled."
            }
            let result = try await tools.migrateToAssets()
            return "Moved \(result.moved) images, rewrote \(result.rewritten) references."
        }
    }

    @objc private func checkTypora() {
        runTool { tools in
            let warnings = await tools.checkTyporaConfig()
            return warnings.joined(separator: "\n")
        }
    }

    private func confirmMigrate(count: Int) async -> Bool {
        await withCheckedContinuation { continuation in
            let alert = NSAlert()
            alert.messageText = "Migrate images?"
            alert.informativeText = "\(count) file(s) may be moved to _assets/."
            alert.addButton(withTitle: "Migrate")
            alert.addButton(withTitle: "Cancel")
            alert.beginSheetModal(for: view.window!) { response in
                continuation.resume(returning: response == .alertFirstButtonReturn)
            }
        }
    }

    private func runTool(_ work: @escaping (NotesImageTools) async throws -> String) {
        runningTask?.cancel()
        setButtonsEnabled(false)
        runningTask = Task {
            do {
                let tools = NotesImageTools(root: root)
                let text = try await work(tools)
                setResults(text)
            } catch {
                setResults("Error: \(error.localizedDescription)")
            }
            setButtonsEnabled(true)
        }
    }
}
