import AppKit
import LumaCore
import LumaModules

enum LauncherModuleLabel {
    static func shortName(for module: ModuleIdentifier) -> String {
        if let badge = ModuleRegistry.presentation(for: module)?.listBadge {
            return badge
        }
        return module.rawValue.replacingOccurrences(of: "luma.", with: "")
    }
}

@MainActor
final class LauncherListRow: NSControl {
    private let item: ResultItem
    private let moduleLabel: String
    private let onRun: (ResultItem) -> Void
    private let onRightClick: ((ResultItem) -> Void)?
    private let onHover: (() -> Void)?
    private let treeGuideView = ListTreeGuideView()
    private let iconView = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(labelWithString: "")
    private let trailingLabel = NSTextField(labelWithString: "")
    private let returnHintContainer = NSView()
    private let returnHint = NSTextField(labelWithString: "↩")
    private let backgroundView = NSView()
    private var selectionAccentLayer: CALayer?
    private var isSelected = false
    private var isHovered = false
    private var hidesTrailingModuleLabel = false
    private var trackingArea: NSTrackingArea?

    init(
        item: ResultItem,
        moduleLabel: String,
        isSelected: Bool,
        hidesTrailingModuleLabel: Bool = false,
        onRun: @escaping (ResultItem) -> Void,
        onRightClick: ((ResultItem) -> Void)? = nil,
        onHover: (() -> Void)? = nil
    ) {
        self.item = item
        self.moduleLabel = moduleLabel
        self.onRun = onRun
        self.onRightClick = onRightClick
        self.onHover = onHover
        self.hidesTrailingModuleLabel = hidesTrailingModuleLabel
        super.init(frame: .zero)
        clipsToBounds = true

        backgroundView.translatesAutoresizingMaskIntoConstraints = false
        backgroundView.wantsLayer = true
        backgroundView.layer?.cornerRadius = LauncherChromeTokens.listRowCornerRadius
        backgroundView.layer?.cornerCurve = .continuous
        backgroundView.layer?.masksToBounds = true
        addSubview(backgroundView)
        NSLayoutConstraint.activate([
            backgroundView.topAnchor.constraint(equalTo: topAnchor),
            backgroundView.leadingAnchor.constraint(equalTo: leadingAnchor),
            backgroundView.trailingAnchor.constraint(equalTo: trailingAnchor),
            backgroundView.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
        setup()
        configureReturnHint()
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

    func setHidesTrailingModuleLabel(_ hidden: Bool) {
        hidesTrailingModuleLabel = hidden
        trailingLabel.isHidden = hidden || isSelected || item.listNest != .none
    }

    func setSelected(_ selected: Bool) {
        isSelected = selected
        let showsHint = selected && item.rowKind != .informational
        returnHintContainer.isHidden = !showsHint
        trailingLabel.isHidden = hidesTrailingModuleLabel || item.listNest != .none || selected
        selectionAccentLayer?.isHidden = !selected
        refreshRowAppearance()
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea { removeTrackingArea(trackingArea) }
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        trackingArea = area
    }

    override func mouseEntered(with event: NSEvent) {
        isHovered = true
        onHover?()
        refreshRowAppearance()
    }

    override func mouseExited(with event: NSEvent) {
        isHovered = false
        refreshRowAppearance()
    }

    private func refreshRowAppearance() {
        guard let layer = backgroundView.layer else { return }
        if isSelected {
            backgroundView.layer?.masksToBounds = false
            layer.backgroundColor = ColorTokens.listRowSelectionFill.cgColor
            layer.borderWidth = 0
            layer.shadowColor = NSColor.controlAccentColor.withAlphaComponent(0.22).cgColor
            layer.shadowOpacity = 0.28
            layer.shadowRadius = LauncherChromeTokens.listRowSelectedShadowRadius
            layer.shadowOffset = CGSize(width: 0, height: -1)
        } else if isHovered {
            backgroundView.layer?.masksToBounds = true
            layer.backgroundColor = ColorTokens.listRowHoverFill.cgColor
            layer.borderWidth = 0
            layer.shadowOpacity = 0
        } else {
            backgroundView.layer?.masksToBounds = true
            layer.backgroundColor = NSColor.clear.cgColor
            layer.borderWidth = 0
            layer.shadowOpacity = 0
        }
    }

    override func layout() {
        super.layout()
        backgroundView.geekLayoutAccentLayers()
    }

    @objc private func run() {
        onRun(item)
    }

    private func configureReturnHint() {
        switch item.rowKind {
        case .informational:
            returnHint.stringValue = ""
        case .actionable:
            returnHint.stringValue = "↩"
        }
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false
        let isNested = item.listNest != .none
        let rowHeight: CGFloat = switch item.displayDensity {
        case .compact: isNested ? LauncherChromeTokens.listRowHeightNested : LauncherChromeTokens.listRowHeight
        case .regular: 58
        case .expanded: 74
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
        titleLabel.font = .systemFont(ofSize: isNested ? 13 : 15, weight: isNested ? .regular : .semibold)
        titleLabel.textColor = isNested ? .secondaryLabelColor : .labelColor
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.maximumNumberOfLines = 1
        titleLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        titleLabel.setContentHuggingPriority(.defaultLow, for: .horizontal)
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        let hasSubtitle = !(item.subtitle ?? "").isEmpty
        subtitleLabel.stringValue = item.subtitle ?? ""
        subtitleLabel.font = .systemFont(ofSize: isNested ? 11 : 12)
        subtitleLabel.textColor = isNested ? .tertiaryLabelColor : .secondaryLabelColor
        subtitleLabel.lineBreakMode = .byTruncatingTail
        subtitleLabel.maximumNumberOfLines = item.displayDensity == .expanded ? 2 : 1
        subtitleLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        subtitleLabel.setContentHuggingPriority(.defaultLow, for: .horizontal)
        subtitleLabel.isHidden = !hasSubtitle
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false

        trailingLabel.stringValue = isNested ? "" : "· \(moduleLabel)"
        trailingLabel.isHidden = hidesTrailingModuleLabel || isNested || isSelected
        trailingLabel.font = TypographyTokens.monoCaption()
        trailingLabel.textColor = .secondaryLabelColor
        trailingLabel.lineBreakMode = .byTruncatingTail
        trailingLabel.maximumNumberOfLines = 1
        trailingLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        trailingLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        trailingLabel.isBezeled = false
        trailingLabel.isEditable = false
        trailingLabel.drawsBackground = false
        trailingLabel.translatesAutoresizingMaskIntoConstraints = false

        returnHint.font = TypographyTokens.monoCaption(weight: .semibold)
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

        selectionAccentLayer = GeekUIKit.installSidebarAccent(on: backgroundView)

        returnHintContainer.addSubview(returnHint)
        if item.listNest != .none {
            addSubview(treeGuideView)
        }
        addSubview(iconView)
        addSubview(titleLabel)
        addSubview(subtitleLabel)
        addSubview(trailingLabel)
        addSubview(returnHintContainer)

        let iconSize: CGFloat = isNested
            ? LauncherChromeTokens.listRowIconSizeNested
            : LauncherChromeTokens.listRowIconSize
        let leadingInset: CGFloat = isNested ? 22 : 4
        let treeGuideWidth: CGFloat = 18
        let titleGap: CGFloat = isNested ? 8 : 10
        let trailingChromeInset: CGFloat = isNested ? 10 : 48

        var constraints: [NSLayoutConstraint] = [
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: leadingInset),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: iconSize),
            iconView.heightAnchor.constraint(equalToConstant: iconSize),
            titleLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: titleGap),
            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            trailingLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            returnHintContainer.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            returnHint.leadingAnchor.constraint(equalTo: returnHintContainer.leadingAnchor, constant: 8),
            returnHint.trailingAnchor.constraint(equalTo: returnHintContainer.trailingAnchor, constant: -8),
            returnHint.topAnchor.constraint(equalTo: returnHintContainer.topAnchor, constant: 4),
            returnHint.bottomAnchor.constraint(equalTo: returnHintContainer.bottomAnchor, constant: -4)
        ]

        if isNested {
            constraints += [
                titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -trailingChromeInset),
                subtitleLabel.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -trailingChromeInset)
            ]
        } else {
            constraints += [
                titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: trailingLabel.leadingAnchor, constant: -8),
                subtitleLabel.trailingAnchor.constraint(lessThanOrEqualTo: trailingLabel.leadingAnchor, constant: -8)
            ]
        }

        if hasSubtitle && !isNested {
            constraints.append(trailingLabel.centerYAnchor.constraint(equalTo: titleLabel.centerYAnchor))
            constraints.append(returnHintContainer.centerYAnchor.constraint(equalTo: titleLabel.centerYAnchor))
        } else {
            constraints.append(trailingLabel.centerYAnchor.constraint(equalTo: centerYAnchor))
            constraints.append(returnHintContainer.centerYAnchor.constraint(equalTo: centerYAnchor))
        }

        if hasSubtitle {
            let topPadding: CGFloat = isNested ? 4 : (item.displayDensity == .compact ? 7 : 8)
            let bottomPadding: CGFloat = isNested ? 4 : 8
            constraints += [
                titleLabel.topAnchor.constraint(equalTo: topAnchor, constant: topPadding),
                subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: isNested ? 1 : 3),
                subtitleLabel.bottomAnchor.constraint(lessThanOrEqualTo: bottomAnchor, constant: -bottomPadding)
            ]
        } else {
            constraints.append(titleLabel.centerYAnchor.constraint(equalTo: iconView.centerYAnchor))
        }

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
    private let divider = NSView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let shortcutLabel = NSTextField(labelWithString: "")

    init(title: String, shortcutIndex: Int?) {
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false

        divider.wantsLayer = true
        divider.layer?.backgroundColor = NSColor.separatorColor.withAlphaComponent(0.55).cgColor
        divider.translatesAutoresizingMaskIntoConstraints = false

        titleLabel.stringValue = title
        titleLabel.font = TypographyTokens.caption2(weight: .semibold)
        titleLabel.textColor = .secondaryLabelColor
        titleLabel.isBezeled = false
        titleLabel.isEditable = false
        titleLabel.drawsBackground = false
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.maximumNumberOfLines = 1
        titleLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        titleLabel.setContentHuggingPriority(.defaultLow, for: .horizontal)
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        if let shortcutIndex {
            shortcutLabel.stringValue = "⌘\(shortcutIndex)"
            shortcutLabel.font = TypographyTokens.monoCaption()
            shortcutLabel.textColor = .tertiaryLabelColor
        }
        shortcutLabel.isBezeled = false
        shortcutLabel.isEditable = false
        shortcutLabel.drawsBackground = false
        shortcutLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        shortcutLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        shortcutLabel.translatesAutoresizingMaskIntoConstraints = false

        addSubview(divider)
        addSubview(titleLabel)
        addSubview(shortcutLabel)
        heightAnchor.constraint(equalToConstant: LauncherChromeTokens.sectionHeaderHeight).isActive = true

        NSLayoutConstraint.activate([
            divider.topAnchor.constraint(equalTo: topAnchor),
            divider.leadingAnchor.constraint(equalTo: leadingAnchor),
            divider.trailingAnchor.constraint(equalTo: trailingAnchor),
            divider.heightAnchor.constraint(equalToConstant: 0.5),

            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 6),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            titleLabel.trailingAnchor.constraint(lessThanOrEqualTo: shortcutLabel.leadingAnchor, constant: -8),
            shortcutLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -6),
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
        label.font = TypographyTokens.body
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
