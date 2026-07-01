import AppKit
import LumaCore

/// Shared light-geek AppKit styling: glass panels, keycaps, action buttons, accent strips.
@MainActor
enum GeekUIKit {
    static func installAccentStrip(on view: NSView, color: NSColor) -> CALayer {
        let strip = CALayer()
        strip.name = "geekAccentStrip"
        strip.backgroundColor = color.cgColor
        strip.cornerRadius = 1.5
        strip.cornerCurve = .continuous
        view.layer?.addSublayer(strip)
        return strip
    }

    static func layoutAccentStrip(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 0,
            y: 0,
            width: GeekStyleTokens.accentStripWidth,
            height: bounds.height
        )
    }

    static func installSidebarAccent(on row: NSView) -> CALayer {
        row.wantsLayer = true
        let strip = CALayer()
        strip.name = "geekSidebarAccent"
        strip.backgroundColor = NSColor.controlAccentColor.cgColor
        strip.cornerRadius = 1
        strip.cornerCurve = .continuous
        strip.isHidden = true
        row.layer?.addSublayer(strip)
        return strip
    }

    static func layoutSidebarAccent(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 2,
            y: 6,
            width: GeekStyleTokens.sidebarAccentWidth,
            height: max(0, bounds.height - 12)
        )
    }

    static func configureGlassPanel(_ panel: NSView, accentHex: String?) {
        panel.wantsLayer = true
        panel.layer?.cornerRadius = GeekStyleTokens.panelCornerRadius
        panel.layer?.cornerCurve = .continuous
        panel.layer?.borderWidth = 0.5
        panel.layer?.borderColor = NSColor.white.withAlphaComponent(GeekStyleTokens.glassPanelBorderAlpha).cgColor
        panel.layer?.backgroundColor = NSColor.clear.cgColor
        panel.layer?.masksToBounds = true

        if panel.subviews.contains(where: { ($0 as? NSVisualEffectView)?.identifier?.rawValue == "geekGlass" }) {
            return
        }

        let glass = NSVisualEffectView()
        glass.identifier = NSUserInterfaceItemIdentifier("geekGlass")
        glass.material = .contentBackground
        glass.blendingMode = .withinWindow
        glass.state = .active
        glass.translatesAutoresizingMaskIntoConstraints = false
        panel.addSubview(glass, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            glass.topAnchor.constraint(equalTo: panel.topAnchor),
            glass.leadingAnchor.constraint(equalTo: panel.leadingAnchor),
            glass.trailingAnchor.constraint(equalTo: panel.trailingAnchor),
            glass.bottomAnchor.constraint(equalTo: panel.bottomAnchor)
        ])

        if let accentHex {
            _ = installAccentStrip(on: panel, color: ColorTokens.color(hex: accentHex))
        }
    }

    static func layoutPanelAccentStrip(on panel: NSView) {
        guard let strip = panel.layer?.sublayers?.first(where: { $0.name == "geekAccentStrip" }) else { return }
        layoutAccentStrip(strip, in: panel.bounds)
    }

    static func configureLightKeycap(host: NSView, label: NSTextField, text: String) {
        host.wantsLayer = true
        host.layer?.cornerRadius = GeekStyleTokens.keycapCornerRadius
        host.layer?.cornerCurve = .continuous
        host.layer?.backgroundColor = NSColor.quaternaryLabelColor.withAlphaComponent(0.55).cgColor
        host.layer?.borderWidth = 0.5
        host.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.65).cgColor

        label.stringValue = text
        label.font = GeekStyleTokens.mono(size: 10, weight: .semibold)
        label.textColor = .secondaryLabelColor
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.alignment = .center
    }

    static func configureKeycap(host: NSView, label: NSTextField, text: String) {
        host.wantsLayer = true
        host.layer?.cornerRadius = GeekStyleTokens.keycapCornerRadius
        host.layer?.cornerCurve = .continuous
        host.layer?.backgroundColor = NSColor.white.withAlphaComponent(GeekStyleTokens.keycapBackgroundAlpha).cgColor
        host.layer?.borderWidth = 0.5
        host.layer?.borderColor = NSColor.white.withAlphaComponent(GeekStyleTokens.keycapBorderAlpha).cgColor

        label.stringValue = text
        label.font = GeekStyleTokens.mono(size: 10, weight: .semibold)
        label.textColor = NSColor.white.withAlphaComponent(0.88)
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.alignment = .center
    }

    static func stylePrimaryButton(_ button: NSButton) {
        button.bezelStyle = .rounded
        button.font = .systemFont(ofSize: 13, weight: .semibold)
        button.contentTintColor = .white
        button.bezelColor = .controlAccentColor
    }

    static func styleSecondaryButton(_ button: NSButton) {
        button.bezelStyle = .rounded
        button.font = TypographyTokens.caption(weight: .medium)
        button.contentTintColor = .labelColor
        button.bezelColor = NSColor.secondaryLabelColor.withAlphaComponent(0.12)
    }

    static func styleToolbarButton(_ button: NSButton) {
        button.bezelStyle = .rounded
        button.controlSize = .small
        button.font = TypographyTokens.caption(weight: .medium)
        button.contentTintColor = .labelColor
        button.bezelColor = NSColor.secondaryLabelColor.withAlphaComponent(0.10)
    }

    static func styleIconToolbarButton(_ button: NSButton, symbol: String, tooltip: String? = nil) {
        button.image = NSImage(systemSymbolName: symbol, accessibilityDescription: tooltip)
        button.bezelStyle = .regularSquare
        button.isBordered = false
        button.contentTintColor = .secondaryLabelColor
        button.toolTip = tooltip
    }

    static func styleDetailBackButton(_ button: NSButton) {
        button.image = NSImage(systemSymbolName: "chevron.left", accessibilityDescription: nil)
        button.imagePosition = .imageLeading
        button.title = "Back"
        button.bezelStyle = .regularSquare
        button.isBordered = false
        button.font = TypographyTokens.caption(weight: .medium)
        button.contentTintColor = .secondaryLabelColor
    }

    static func styleDetailCloseButton(_ button: NSButton) {
        button.image = NSImage(systemSymbolName: "xmark", accessibilityDescription: "Close")
        button.bezelStyle = .regularSquare
        button.isBordered = false
        button.contentTintColor = .tertiaryLabelColor
    }

    static func styleDetailSearchField(_ field: NSSearchField) {
        field.font = TypographyTokens.body
        field.focusRingType = .none
    }

    static func configureEmptyStateLabel(_ label: NSTextField, text: String) {
        label.stringValue = text
        label.font = TypographyTokens.body
        label.textColor = .secondaryLabelColor
        label.alignment = .center
        label.maximumNumberOfLines = 3
        label.lineBreakMode = .byWordWrapping
    }

    static func configureStatusLabel(_ label: NSTextField) {
        label.font = TypographyTokens.caption()
        label.textColor = .secondaryLabelColor
        label.lineBreakMode = .byTruncatingTail
    }

    static func configureDetailSeparator(_ box: NSBox) {
        box.boxType = .separator
        box.fillColor = NSColor.separatorColor.withAlphaComponent(0.45)
    }

    static func configureContentSurface(_ view: NSView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = LauncherChromeTokens.detailSurfaceCornerRadius
        view.layer?.cornerCurve = .continuous
        view.layer?.borderWidth = 0.5
        view.layer?.borderColor = NSColor.separatorColor
            .withAlphaComponent(LauncherChromeTokens.detailSurfaceBorderAlpha).cgColor
        view.layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.18).cgColor
    }

    static func installDetailRootChrome(on view: NSView) {
        view.wantsLayer = true
        view.layer?.backgroundColor = NSColor.clear.cgColor
        view.layer?.masksToBounds = true
    }

    static func makeDetailSectionCard(header: String, contentViews: [NSView]) -> NSView {
        let headerLabel = NSTextField(labelWithString: header.uppercased())
        styleDetailSectionHeaderLabel(headerLabel, title: header.uppercased())

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.spacing = 6
        stack.alignment = .leading
        stack.setContentHuggingPriority(.defaultHigh, for: .vertical)
        stack.addArrangedSubview(headerLabel)
        for contentView in contentViews {
            stack.addArrangedSubview(contentView)
            contentView.translatesAutoresizingMaskIntoConstraints = false
            contentView.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        }

        let card = NSView()
        configureContentSurface(card)
        stack.translatesAutoresizingMaskIntoConstraints = false
        card.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: card.topAnchor, constant: 10),
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -12),
            stack.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -10)
        ])
        return card
    }

    static func configureDetailTableColumn(
        _ column: NSTableColumn,
        minWidth: CGFloat,
        maxWidth: CGFloat = 2000,
        resizingMask: NSTableColumn.ResizingOptions = [.autoresizingMask, .userResizingMask]
    ) {
        column.minWidth = minWidth
        column.maxWidth = maxWidth
        column.resizingMask = resizingMask
    }

    static func makeDetailTableCell(
        text: String,
        font: NSFont = .systemFont(ofSize: 12),
        color: NSColor = .labelColor,
        lineBreak: NSLineBreakMode = .byTruncatingTail,
        toolTip: String? = nil
    ) -> NSTableCellView {
        let cell = NSTableCellView()
        let label = NSTextField(labelWithString: text)
        label.font = font
        label.textColor = color
        label.lineBreakMode = lineBreak
        label.translatesAutoresizingMaskIntoConstraints = false
        label.toolTip = toolTip
        cell.addSubview(label)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
            label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
            label.centerYAnchor.constraint(equalTo: cell.centerYAnchor)
        ])
        cell.toolTip = toolTip
        return cell
    }

    static func installSearchSurface(on view: NSView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = LauncherChromeTokens.searchBarCornerRadius
        view.layer?.cornerCurve = .continuous
        view.layer?.borderWidth = 0.5
        view.layer?.borderColor = ColorTokens.searchSurfaceBorder.cgColor
        view.layer?.backgroundColor = ColorTokens.searchSurfaceFill.cgColor
    }

    static func installPerformanceStripSurface(on view: NSView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = LauncherChromeTokens.performanceMetricCornerRadius
        view.layer?.cornerCurve = .continuous
        view.layer?.backgroundColor = ColorTokens.performanceStripSurfaceFill.cgColor
    }

    static func configureDetailTable(_ tableView: NSTableView, rowHeight: CGFloat = LauncherChromeTokens.detailTableRowHeight) {
        tableView.style = .plain
        tableView.rowHeight = rowHeight
        tableView.backgroundColor = .clear
        tableView.selectionHighlightStyle = .regular
        tableView.usesAlternatingRowBackgroundColors = false
        tableView.intercellSpacing = NSSize(width: 0, height: LauncherChromeTokens.detailTableRowSpacing)
        styleDetailTableColumns(tableView)
    }

    static func styleDetailTableColumns(_ tableView: NSTableView) {
        for column in tableView.tableColumns {
            let headerCell = column.headerCell
            headerCell.font = TypographyTokens.caption2(weight: .semibold)
            headerCell.textColor = .secondaryLabelColor
        }
    }

    static func configureDetailTableRowSurface(_ view: NSView) {
        view.wantsLayer = true
        view.layer?.cornerRadius = LauncherChromeTokens.detailTableRowCornerRadius
        view.layer?.cornerCurve = .continuous
        view.layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.10).cgColor
    }

    static func styleDetailSectionHeaderLabel(_ label: NSTextField, title: String) {
        label.stringValue = title
        label.font = TypographyTokens.caption2(weight: .semibold)
        label.textColor = .tertiaryLabelColor
    }

    static func makeToolbarButton(_ title: String, target: AnyObject?, action: Selector) -> NSButton {
        let button = NSButton(title: title, target: target, action: action)
        styleToolbarButton(button)
        return button
    }

    static func styleLanguageChip(_ chip: NSButton, selected: Bool) {
        chip.bezelStyle = .rounded
        chip.font = .systemFont(ofSize: 11, weight: .medium)
        if selected {
            chip.bezelColor = .controlAccentColor
            chip.contentTintColor = .white
            chip.alphaValue = 1
        } else {
            chip.bezelColor = NSColor.secondaryLabelColor.withAlphaComponent(0.14)
            chip.contentTintColor = .labelColor
            chip.alphaValue = 0.9
        }
    }
}

extension NSView {
    func geekLayoutAccentLayers() {
        guard let sublayers = layer?.sublayers else { return }
        for sublayer in sublayers where sublayer.name == "geekAccentStrip" {
            GeekUIKit.layoutAccentStrip(sublayer, in: bounds)
        }
        for sublayer in sublayers where sublayer.name == "geekSidebarAccent" {
            GeekUIKit.layoutSidebarAccent(sublayer, in: bounds)
        }
    }
}

@MainActor
final class GeekGlassPanel: NSView {
    init(accentHex: String?) {
        super.init(frame: .zero)
        GeekUIKit.configureContentSurface(self)
        if let accentHex {
            _ = GeekUIKit.installAccentStrip(on: self, color: ColorTokens.color(hex: accentHex))
        }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layout() {
        super.layout()
        geekLayoutAccentLayers()
    }
}
