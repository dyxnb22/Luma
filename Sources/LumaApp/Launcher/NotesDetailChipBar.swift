import AppKit
import LumaModules

enum NotesDetailChip: Int, CaseIterable {
    case today = 0
    case inbox
    case recent
    case pinned

    var title: String {
        switch self {
        case .today: return "Today"
        case .inbox: return "Inbox"
        case .recent: return "Recent"
        case .pinned: return "Pinned"
        }
    }
}

enum NotesDetailPanel: Int, CaseIterable {
    case outline = 0
    case browse
    case inbox

    var title: String {
        switch self {
        case .outline: return "Outline"
        case .browse: return "Browse"
        case .inbox: return "Inbox"
        }
    }
}

@MainActor
final class NotesDetailChipBar: NSView {
    var onChipChanged: ((NotesDetailChip?) -> Void)?
    var onPanelChanged: ((NotesDetailPanel) -> Void)?

    private let chipControl = NSSegmentedControl()
    private let panelControl = NSSegmentedControl()
    private var selectedChip: NotesDetailChip?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    func setInboxCount(_ count: Int) {
        chipControl.setLabel(count > 0 ? "Inbox(\(count))" : "Inbox", forSegment: NotesDetailChip.inbox.rawValue)
    }

    func setTodayHint(missing: Bool) {
        chipControl.setLabel(missing ? "Today +" : "Today", forSegment: NotesDetailChip.today.rawValue)
    }

    func selectChip(_ chip: NotesDetailChip?) {
        selectedChip = chip
        chipControl.selectedSegment = chip?.rawValue ?? -1
    }

    func selectPanel(_ panel: NotesDetailPanel) {
        panelControl.selectedSegment = panel.rawValue
    }

    func currentPanel() -> NotesDetailPanel {
        NotesDetailPanel(rawValue: panelControl.selectedSegment) ?? .outline
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false

        chipControl.segmentCount = NotesDetailChip.allCases.count
        for chip in NotesDetailChip.allCases {
            chipControl.setLabel(chip.title, forSegment: chip.rawValue)
        }
        chipControl.segmentStyle = .rounded
        chipControl.trackingMode = .selectOne
        chipControl.selectedSegment = -1
        chipControl.target = self
        chipControl.action = #selector(chipChanged)
        chipControl.translatesAutoresizingMaskIntoConstraints = false

        panelControl.segmentCount = NotesDetailPanel.allCases.count
        for panel in NotesDetailPanel.allCases {
            panelControl.setLabel(panel.title, forSegment: panel.rawValue)
        }
        panelControl.segmentStyle = .rounded
        panelControl.selectedSegment = NotesDetailPanel.outline.rawValue
        panelControl.target = self
        panelControl.action = #selector(panelChanged)
        panelControl.translatesAutoresizingMaskIntoConstraints = false

        addSubview(chipControl)
        addSubview(panelControl)

        NSLayoutConstraint.activate([
            chipControl.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            chipControl.centerYAnchor.constraint(equalTo: centerYAnchor),

            panelControl.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            panelControl.centerYAnchor.constraint(equalTo: centerYAnchor),
            panelControl.leadingAnchor.constraint(greaterThanOrEqualTo: chipControl.trailingAnchor, constant: 12),

            heightAnchor.constraint(equalToConstant: 28)
        ])
    }

    @objc private func chipChanged() {
        if chipControl.selectedSegment < 0 {
            selectedChip = nil
            onChipChanged?(nil)
            return
        }
        let chip = NotesDetailChip(rawValue: chipControl.selectedSegment)
        selectedChip = chip
        onChipChanged?(chip)
    }

    @objc private func panelChanged() {
        onPanelChanged?(currentPanel())
    }
}
