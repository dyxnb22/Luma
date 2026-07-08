@preconcurrency import AppKit
import LumaCore

enum LauncherHintContext: String {
    case home
    case results
    case detail
}

// AppKit display cycle calls layout() without Swift MainActor executor — do not isolate this view.
final class LauncherHintBar: NSView {
    private let leftLabel = NSTextField(labelWithString: "")
    private let statusLabel = NSTextField(labelWithString: "")
    private(set) var currentHintContext: LauncherHintContext = .home

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        clipsToBounds = true

        leftLabel.font = TypographyTokens.monoCaption()
        leftLabel.textColor = Self.hintTextColor
        leftLabel.lineBreakMode = .byTruncatingTail
        leftLabel.maximumNumberOfLines = 1
        leftLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        leftLabel.isBezeled = false
        leftLabel.isEditable = false
        leftLabel.drawsBackground = false
        leftLabel.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.font = TypographyTokens.caption2()
        statusLabel.textColor = Self.hintTextColor
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.maximumNumberOfLines = 1
        statusLabel.setContentHuggingPriority(.required, for: .horizontal)
        statusLabel.alignment = .right
        statusLabel.isBezeled = false
        statusLabel.isEditable = false
        statusLabel.drawsBackground = false
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(leftLabel)
        addSubview(statusLabel)
        heightAnchor.constraint(equalToConstant: LauncherChromeTokens.hintBarHeight).isActive = true

        NSLayoutConstraint.activate([
            leftLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            leftLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            statusLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            statusLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            statusLabel.leadingAnchor.constraint(greaterThanOrEqualTo: leftLabel.trailingAnchor, constant: 12)
        ])

        setContext(.home)
        setModulesReady(true)
    }

    nonisolated override func layout() {
        super.layout()
        let maxWidth = max(0, bounds.width - statusLabel.intrinsicContentSize.width - 24)
        leftLabel.preferredMaxLayoutWidth = maxWidth
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    @MainActor
    func setContext(_ context: LauncherHintContext, selectedItem: ResultItem? = nil) {
        currentHintContext = context
        switch context {
        case .home, .results:
            leftLabel.stringValue = keyHints(for: context, selectedItem: selectedItem)
        case .detail:
            leftLabel.stringValue = L10n.tr("hint.detail.back")
        }
    }

    @MainActor
    private func keyHints(for context: LauncherHintContext, selectedItem: ResultItem?) -> String {
        let escLabel = context == .home ? L10n.tr("hint.escClose") : L10n.tr("hint.escClear")
        var parts = [L10n.tr("hint.select"), L10n.tr("hint.open")]
        if let secondary = selectedItem?.secondaryActions.first {
            parts.append(L10n.tr("hint.more"))
            switch secondary.confirmation {
            case .requireSecondModifier:
                parts.append("⌘↩ \(shortActionLabel(secondary.title)) · confirm")
            case .requireReturn:
                parts.append("⌘↩ \(shortActionLabel(secondary.title)) · confirm")
            case .none:
                parts.append("⌘↩ \(shortActionLabel(secondary.title))")
            }
        } else {
            parts.append(L10n.tr("hint.more"))
        }
        parts.append(escLabel)
        return parts.joined(separator: "    ")
    }

    @MainActor
    private func shortActionLabel(_ title: String) -> String {
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.count <= 18 { return trimmed }
        return String(trimmed.prefix(16)) + "…"
    }

    @MainActor
    func setModulesReady(_ ready: Bool) {
        statusLabel.stringValue = ready ? "" : "Loading…"
        statusLabel.isHidden = ready
    }

    private static var hintTextColor: NSColor {
        .labelColor.withAlphaComponent(0.78)
    }
}
