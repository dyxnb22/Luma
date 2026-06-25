import AppKit
import LumaCore
import LumaModules

enum LauncherModuleLabel {
    static func shortName(for module: ModuleIdentifier) -> String {
        switch module {
        case .apps: "apps"
        case .clipboard: "clip"
        case .commands: "cmd"
        case .notes: "notes"
        case .todo: "todo"
        case .events: "events"
        case .translate: "tr"
        case .wordbook: "word"
        case .snippets: "snip"
        case .secrets: "vault"
        case .media: "media"
        case .windows: "win"
        case .calculator: "calc"
        case .windowLayouts: "layout"
        default: module.rawValue.replacingOccurrences(of: "luma.", with: "")
        }
    }
}

@MainActor
final class LauncherListRow: NSControl {
    private let item: ResultItem
    private let moduleLabel: String
    private let onRun: (ResultItem) -> Void
    private let onRightClick: ((ResultItem) -> Void)?
    private let treeGuideView = ListTreeGuideView()
    private let iconView = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(labelWithString: "")
    private let trailingLabel = NSTextField(labelWithString: "")
    private let returnHintContainer = NSView()
    private let returnHint = NSTextField(labelWithString: "↩")
    private var selectionAccentLayer: CALayer?

    init(
        item: ResultItem,
        moduleLabel: String,
        isSelected: Bool,
        onRun: @escaping (ResultItem) -> Void,
        onRightClick: ((ResultItem) -> Void)? = nil
    ) {
        self.item = item
        self.moduleLabel = moduleLabel
        self.onRun = onRun
        self.onRightClick = onRightClick
        super.init(frame: .zero)
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.cornerCurve = .continuous
        setup()
        setSelected(isSelected)
        target = self
        action = #selector(run)
        setAccessibilityRole(.button)
        setAccessibilityLabel(item.title)
        if let subtitle = item.subtitle, !subtitle.isEmpty {
            setAccessibilityHelp(subtitle)
        }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func mouseDown(with event: NSEvent) {
        if event.type == .rightMouseDown || event.modifierFlags.contains(.control) {
            onRightClick?(item)
            return
        }
        onRun(item)
    }

    override func rightMouseDown(with event: NSEvent) {
        onRightClick?(item)
    }

    func setSelected(_ isSelected: Bool) {
        layer?.backgroundColor = isSelected
            ? NSColor.controlAccentColor.withAlphaComponent(0.06).cgColor
            : NSColor.clear.cgColor
        returnHintContainer.isHidden = !isSelected
        trailingLabel.isHidden = isSelected
        selectionAccentLayer?.isHidden = !isSelected
    }

    override func layout() {
        super.layout()
        geekLayoutAccentLayers()
    }

    @objc private func run() {
        onRun(item)
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false
        let isNested = item.listNest != .none
        let rowHeight: CGFloat = switch item.displayDensity {
        case .compact: isNested ? 38 : 44
        case .regular: 56
        case .expanded: 72
        }
        heightAnchor.constraint(equalToConstant: rowHeight).isActive = true

        if case .child(let isLast) = item.listNest {
            treeGuideView.isLast = isLast
            treeGuideView.translatesAutoresizingMaskIntoConstraints = false
        }

        iconView.image = Self.iconImage(for: item.icon, nested: isNested)
        iconView.imageScaling = .scaleProportionallyDown
        iconView.wantsLayer = true
        iconView.layer?.cornerRadius = isNested ? 4 : 8
        iconView.layer?.cornerCurve = .continuous
        iconView.layer?.masksToBounds = true
        iconView.contentTintColor = isNested ? .secondaryLabelColor : nil
        iconView.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.stringValue = item.title
        titleLabel.font = .systemFont(ofSize: isNested ? 13 : 15, weight: isNested ? .regular : .medium)
        titleLabel.textColor = isNested ? .secondaryLabelColor : .labelColor
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        subtitleLabel.stringValue = item.subtitle ?? ""
        subtitleLabel.font = .systemFont(ofSize: isNested ? 11 : 12)
        subtitleLabel.textColor = isNested ? .tertiaryLabelColor : .secondaryLabelColor
        subtitleLabel.lineBreakMode = item.displayDensity == .expanded ? .byTruncatingMiddle : .byTruncatingTail
        subtitleLabel.maximumNumberOfLines = item.displayDensity == .expanded ? 2 : 1
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false

        trailingLabel.stringValue = isNested ? "" : "· \(moduleLabel)"
        trailingLabel.font = GeekStyleTokens.mono(size: 11)
        trailingLabel.textColor = .tertiaryLabelColor
        trailingLabel.isBezeled = false
        trailingLabel.isEditable = false
        trailingLabel.drawsBackground = false
        trailingLabel.translatesAutoresizingMaskIntoConstraints = false

        returnHint.font = GeekStyleTokens.mono(size: 11, weight: .semibold)
        returnHint.textColor = .labelColor
        returnHint.isBezeled = false
        returnHint.isEditable = false
        returnHint.drawsBackground = false
        returnHint.translatesAutoresizingMaskIntoConstraints = false

        returnHintContainer.wantsLayer = true
        returnHintContainer.layer?.cornerRadius = 10
        returnHintContainer.layer?.cornerCurve = .continuous
        returnHintContainer.layer?.backgroundColor = NSColor.controlAccentColor
            .withAlphaComponent(ColorTokens.returnHintCapsuleAlpha).cgColor
        returnHintContainer.isHidden = true
        returnHintContainer.translatesAutoresizingMaskIntoConstraints = false

        selectionAccentLayer = GeekUIKit.installSidebarAccent(on: self)

        returnHintContainer.addSubview(returnHint)
        if item.listNest != .none {
            addSubview(treeGuideView)
        }
        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(subtitleLabel)
        addSubview(trailingLabel)
        addSubview(returnHintContainer)

        let topPadding: CGFloat = isNested ? 4 : (item.displayDensity == .compact ? 6 : 10)
        let iconSize: CGFloat = isNested ? 22 : 36
        let leadingInset: CGFloat = isNested ? 22 : 0
        let treeGuideWidth: CGFloat = 18

        var constraints: [NSLayoutConstraint] = [
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: leadingInset),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: iconSize),
            iconView.heightAnchor.constraint(equalToConstant: iconSize),
            titleLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: isNested ? 8 : 12),
            titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: returnHintContainer.leadingAnchor, constant: -8),
            titleLabel.topAnchor.constraint(equalTo: topAnchor, constant: topPadding),
            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            subtitleLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: isNested ? 1 : 2),
            trailingLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            trailingLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            returnHintContainer.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            returnHintContainer.centerYAnchor.constraint(equalTo: centerYAnchor),
            returnHint.leadingAnchor.constraint(equalTo: returnHintContainer.leadingAnchor, constant: 8),
            returnHint.trailingAnchor.constraint(equalTo: returnHintContainer.trailingAnchor, constant: -8),
            returnHint.topAnchor.constraint(equalTo: returnHintContainer.topAnchor, constant: 4),
            returnHint.bottomAnchor.constraint(equalTo: returnHintContainer.bottomAnchor, constant: -4)
        ]

        if case .child(let isLast) = item.listNest {
            treeGuideView.isLast = isLast
            constraints += [
                treeGuideView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
                treeGuideView.widthAnchor.constraint(equalToConstant: treeGuideWidth),
                treeGuideView.topAnchor.constraint(equalTo: topAnchor, constant: -2),
                treeGuideView.bottomAnchor.constraint(equalTo: bottomAnchor, constant: 2)
            ]
        }

        NSLayoutConstraint.activate(constraints)
    }

    private static func iconImage(for icon: LumaCore.IconRef, nested: Bool) -> NSImage? {
        switch icon {
        case .bundleID(let bundleID):
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID) {
                return IconCache.shared.appIcon(for: url)
            }
            return NSImage(systemSymbolName: "app.fill", accessibilityDescription: nil)
        case .symbol(let symbol):
            let config = nested ? NSImage.SymbolConfiguration(pointSize: 13, weight: .regular) : nil
            return NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
                .withSymbolConfiguration(config ?? .init())
        case .file(let url):
            return IconCache.shared.appIcon(for: url)
        case .none:
            return NSImage(systemSymbolName: "sparkles", accessibilityDescription: nil)
        }
    }
}

@MainActor
private final class ListTreeGuideView: NSView {
    var isLast = false

    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        let stroke = NSColor.separatorColor.withAlphaComponent(0.55)
        stroke.setStroke()

        let trunkX = bounds.width - 4
        let branchEndX = bounds.width + 6
        let midY = bounds.midY

        let vertical = NSBezierPath()
        vertical.lineWidth = 1
        vertical.move(to: NSPoint(x: trunkX, y: 0))
        vertical.line(to: NSPoint(x: trunkX, y: isLast ? midY : bounds.height))
        vertical.stroke()

        let branch = NSBezierPath()
        branch.lineWidth = 1
        branch.move(to: NSPoint(x: trunkX, y: midY))
        branch.line(to: NSPoint(x: branchEndX, y: midY))
        branch.stroke()
    }
}

@MainActor
final class LauncherSectionHeaderView: NSView {
    private let titleLabel = NSTextField(labelWithString: "")
    private let shortcutLabel = NSTextField(labelWithString: "")

    init(title: String, shortcutIndex: Int?) {
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        titleLabel.stringValue = title
        titleLabel.font = .systemFont(ofSize: 11, weight: .semibold)
        titleLabel.textColor = .secondaryLabelColor
        titleLabel.isBezeled = false
        titleLabel.isEditable = false
        titleLabel.drawsBackground = false
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        if let shortcutIndex {
            shortcutLabel.stringValue = "⌘\(shortcutIndex)"
            shortcutLabel.font = GeekStyleTokens.mono(size: 11, weight: .medium)
            shortcutLabel.textColor = .tertiaryLabelColor
        }
        shortcutLabel.isBezeled = false
        shortcutLabel.isEditable = false
        shortcutLabel.drawsBackground = false
        shortcutLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(titleLabel)
        addSubview(shortcutLabel)
        heightAnchor.constraint(equalToConstant: 24).isActive = true

        NSLayoutConstraint.activate([
            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            shortcutLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            shortcutLabel.centerYAnchor.constraint(equalTo: centerYAnchor)
        ])

        setAccessibilityRole(.staticText)
        setAccessibilityLabel(title)
        setAccessibilityChildren([titleLabel, shortcutLabel])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

@MainActor
final class LauncherPlaceholderRow: NSView {
    private let label = NSTextField(labelWithString: "")

    init(text: String) {
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        label.stringValue = text
        label.font = .systemFont(ofSize: 13)
        label.textColor = .tertiaryLabelColor
        label.alignment = .center
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.translatesAutoresizingMaskIntoConstraints = false
        addSubview(label)
        heightAnchor.constraint(equalToConstant: 56).isActive = true
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: centerXAnchor),
            label.centerYAnchor.constraint(equalTo: centerYAnchor),
            label.leadingAnchor.constraint(greaterThanOrEqualTo: leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -16)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}
