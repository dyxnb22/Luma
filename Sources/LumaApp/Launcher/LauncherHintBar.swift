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

        leftLabel.font = TypographyTokens.monoCaption()
        leftLabel.textColor = .tertiaryLabelColor
        leftLabel.isBezeled = false
        leftLabel.isEditable = false
        leftLabel.drawsBackground = false
        leftLabel.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.font = TypographyTokens.caption2()
        statusLabel.textColor = .tertiaryLabelColor
        statusLabel.alignment = .right
        statusLabel.isBezeled = false
        statusLabel.isEditable = false
        statusLabel.drawsBackground = false
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(leftLabel)
        addSubview(statusLabel)
        heightAnchor.constraint(equalToConstant: LauncherChromeTokens.hintBarHeight).isActive = true

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

    func setContext(_ context: LauncherHintContext, selectedItem: ResultItem? = nil) {
        switch context {
        case .home, .results:
            leftLabel.stringValue = keyHints(for: context, selectedItem: selectedItem)
        case .detail:
            leftLabel.stringValue = "Esc Back    ⌘W Close detail"
        }
    }

    private func keyHints(for context: LauncherHintContext, selectedItem: ResultItem?) -> String {
        let escLabel = context == .home ? "Close" : "Clear"
        var parts = ["↩ Open"]
        if let secondary = selectedItem?.secondaryActions.first {
            parts.append("⇥ More")
            parts.append("⌘↩ \(shortActionLabel(secondary.title))")
        } else {
            parts.append("⇥ More")
        }
        parts.append("Esc \(escLabel)")
        return parts.joined(separator: "    ")
    }

    private func shortActionLabel(_ title: String) -> String {
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.count <= 18 { return trimmed }
        return String(trimmed.prefix(16)) + "…"
    }

    func setModulesReady(_ ready: Bool) {
        statusLabel.stringValue = ready ? "" : "Loading…"
        statusLabel.isHidden = ready
    }
}
