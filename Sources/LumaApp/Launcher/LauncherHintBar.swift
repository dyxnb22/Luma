import AppKit
import LumaCore

enum LauncherHintContext {
    case home
    case results
    case detail
}

@MainActor
final class LauncherHintBar: NSView {
    private let leftLabel = NSTextField(labelWithString: "")
    private let statusLabel = NSTextField(labelWithString: "")

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        wantsLayer = true

        leftLabel.font = GeekStyleTokens.mono(size: 11)
        leftLabel.textColor = .tertiaryLabelColor
        leftLabel.isBezeled = false
        leftLabel.isEditable = false
        leftLabel.drawsBackground = false
        leftLabel.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.font = .systemFont(ofSize: 11)
        statusLabel.textColor = .tertiaryLabelColor
        statusLabel.alignment = .right
        statusLabel.isBezeled = false
        statusLabel.isEditable = false
        statusLabel.drawsBackground = false
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(leftLabel)
        addSubview(statusLabel)
        heightAnchor.constraint(equalToConstant: 28).isActive = true

        NSLayoutConstraint.activate([
            leftLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            leftLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            statusLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            statusLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            statusLabel.leadingAnchor.constraint(greaterThanOrEqualTo: leftLabel.trailingAnchor, constant: 12)
        ])

        setContext(.home)
        setModulesReady(true)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func setContext(_ context: LauncherHintContext) {
        switch context {
        case .home:
            leftLabel.stringValue = "↩ Open    ⇥ Actions    ⌘K Actions    Esc Close"
        case .results:
            leftLabel.stringValue = "↩ Open    ⇥ Actions    Esc Clear"
        case .detail:
            leftLabel.stringValue = "Esc Back    ⌘W Close detail"
        }
    }

    func setModulesReady(_ ready: Bool) {
        statusLabel.stringValue = ready ? "" : "Loading…"
        statusLabel.isHidden = ready
    }
}
