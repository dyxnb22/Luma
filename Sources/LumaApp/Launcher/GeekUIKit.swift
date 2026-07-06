@preconcurrency import AppKit
import LumaCore
import ObjectiveC

/// Holds a clip-view resize observer token for `wireVerticalListScroll`.
private final class ScrollClipResizeObserver {
    private let token: NSObjectProtocol

    init(scrollView: NSScrollView, handler: @escaping @MainActor () -> Void) {
        token = NotificationCenter.default.addObserver(
            forName: NSView.frameDidChangeNotification,
            object: scrollView,
            queue: nil
        ) { _ in
            Task { @MainActor in
                handler()
            }
        }
    }

    deinit {
        NotificationCenter.default.removeObserver(token)
    }
}

private enum ScrollClipObserverAssociation {
    nonisolated(unsafe) static var key: UInt8 = 0
}

private extension NSScrollView {
    var clipResizeObserver: ScrollClipResizeObserver? {
        get { objc_getAssociatedObject(self, &ScrollClipObserverAssociation.key) as? ScrollClipResizeObserver }
        set { objc_setAssociatedObject(self, &ScrollClipObserverAssociation.key, newValue, .OBJC_ASSOCIATION_RETAIN_NONATOMIC) }
    }
}
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

    nonisolated static func layoutAccentStrip(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 0,
            y: 0,
            width: 4,
            height: bounds.height
        )
    }

    static func installSidebarAccent(on row: NSView) -> CALayer {
        // Caller must pass a dedicated chrome child (e.g. list row `backgroundView`), not a full-width host.
        if row.layer == nil {
            row.wantsLayer = true
        }
        let strip = CALayer()
        strip.name = "geekSidebarAccent"
        strip.backgroundColor = NSColor.controlAccentColor.cgColor
        strip.cornerRadius = 1.5
        strip.cornerCurve = .continuous
        strip.isHidden = true
        row.layer?.addSublayer(strip)
        return strip
    }

    nonisolated static func layoutSidebarAccent(_ strip: CALayer, in bounds: CGRect) {
        strip.frame = CGRect(
            x: 3,
            y: 8,
            width: 2,
            height: max(0, bounds.height - 16)
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

    private static let contentSurfaceChromeID = "geekContentSurface"

    nonisolated static func contentSurfaceChrome(in view: NSView) -> NSView? {
        view.subviews.first { $0.identifier?.rawValue == "geekContentSurface" }
    }

    static func configureContentSurface(_ view: NSView) {
        guard contentSurfaceChrome(in: view) == nil else { return }
        view.clipsToBounds = true

        let chrome = NSView()
        chrome.identifier = NSUserInterfaceItemIdentifier(contentSurfaceChromeID)
        chrome.translatesAutoresizingMaskIntoConstraints = false
        chrome.wantsLayer = true
        chrome.layer?.cornerRadius = LauncherChromeTokens.detailSurfaceCornerRadius
        chrome.layer?.cornerCurve = .continuous
        chrome.layer?.borderWidth = 0.5
        chrome.layer?.borderColor = ColorTokens.contentSurfaceBorder.cgColor
        chrome.layer?.backgroundColor = ColorTokens.contentSurfaceFill.cgColor
        view.addSubview(chrome, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            chrome.topAnchor.constraint(equalTo: view.topAnchor),
            chrome.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            chrome.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            chrome.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
    }

    static func installDetailRootChrome(on view: NSView) {
        // Full-width detail hosts must not use wantsLayer — same anchorPoint drift as the launcher root.
        view.clipsToBounds = true
    }

    static func installHomeListSurface(on view: NSView) {
        view.clipsToBounds = true
    }

    /// Pinned chrome for the 280 pt Open Apps column (ADR-032). Layer lives on child only.
    static func installHomeListColumnSurface(on view: NSView) {
        let surfaceID = "homeListColumnSurface"
        guard view.subviews.first(where: { $0.identifier?.rawValue == surfaceID }) == nil else { return }
        view.clipsToBounds = true

        let surface = NSView()
        surface.identifier = NSUserInterfaceItemIdentifier(surfaceID)
        surface.translatesAutoresizingMaskIntoConstraints = false
        surface.wantsLayer = true
        surface.layer?.cornerRadius = LauncherChromeTokens.homeListSurfaceCornerRadius
        surface.layer?.cornerCurve = .continuous
        surface.layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.20).cgColor
        surface.layer?.borderWidth = 0.5
        surface.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.18).cgColor
        view.addSubview(surface, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            surface.topAnchor.constraint(equalTo: view.topAnchor),
            surface.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            surface.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            surface.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
    }

    static func installPerformanceStripSurface(on view: NSView) {
        let surfaceID = "performanceStripSurface"
        guard view.subviews.first(where: { $0.identifier?.rawValue == surfaceID }) == nil else { return }
        view.clipsToBounds = true

        let surface = NSView()
        surface.identifier = NSUserInterfaceItemIdentifier(surfaceID)
        surface.translatesAutoresizingMaskIntoConstraints = false
        surface.wantsLayer = true
        surface.layer?.cornerRadius = LauncherChromeTokens.performanceMetricCornerRadius
        surface.layer?.cornerCurve = .continuous
        surface.layer?.backgroundColor = ColorTokens.performanceStripSurfaceFill.cgColor
        view.addSubview(surface, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            surface.topAnchor.constraint(equalTo: view.topAnchor),
            surface.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            surface.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            surface.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
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
        label.maximumNumberOfLines = 1
        label.translatesAutoresizingMaskIntoConstraints = false
        label.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        label.toolTip = toolTip ?? (text.isEmpty ? nil : text)
        cell.addSubview(label)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
            label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
            label.centerYAnchor.constraint(equalTo: cell.centerYAnchor)
        ])
        cell.toolTip = label.toolTip
        return cell
    }

    /// Pins a horizontal action row in `container`, scrolling when buttons overflow.
    @discardableResult
    static func constrainDetailFooterActions(
        _ stack: NSStackView,
        in container: NSView,
        below topView: NSView,
        topSpacing: CGFloat = LauncherChromeTokens.detailSectionGap,
        bottomMargin: CGFloat = LauncherChromeTokens.detailMargin,
        horizontalMargin: CGFloat = LauncherChromeTokens.detailMargin
    ) -> NSScrollView {
        stack.orientation = .horizontal
        stack.spacing = 8
        stack.alignment = .centerY
        stack.translatesAutoresizingMaskIntoConstraints = false

        let scroll = NSScrollView()
        scroll.hasVerticalScroller = false
        scroll.hasHorizontalScroller = true
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = stack

        container.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: topView.bottomAnchor, constant: topSpacing),
            scroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: horizontalMargin),
            scroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -horizontalMargin),
            scroll.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -bottomMargin),
            stack.heightAnchor.constraint(equalTo: scroll.contentView.heightAnchor)
        ])
        return scroll
    }

    /// Pins trailing toolbar buttons so they scroll instead of clipping on narrow panels.
    @discardableResult
    static func constrainDetailToolbarTrailingActions(
        _ stack: NSStackView,
        in toolbar: NSView,
        after leadingView: NSView,
        spacing: CGFloat = 12,
        trailingMargin: CGFloat = 0
    ) -> NSScrollView {
        stack.orientation = .horizontal
        stack.spacing = 8
        stack.alignment = .centerY
        stack.translatesAutoresizingMaskIntoConstraints = false

        let scroll = NSScrollView()
        scroll.hasVerticalScroller = false
        scroll.hasHorizontalScroller = true
        scroll.autohidesScrollers = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = stack

        toolbar.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.leadingAnchor.constraint(equalTo: leadingView.trailingAnchor, constant: spacing),
            scroll.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor, constant: -trailingMargin),
            scroll.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            scroll.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.detailToolbarHeight - 4),
            stack.heightAnchor.constraint(equalTo: scroll.contentView.heightAnchor)
        ])
        leadingView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return scroll
    }

    private static let searchSurfaceChromeID = "geekSearchSurface"

    static func installSearchSurface(on view: NSView) {
        guard view.subviews.first(where: { $0.identifier?.rawValue == searchSurfaceChromeID }) == nil else { return }

        let chrome = NSView()
        chrome.identifier = NSUserInterfaceItemIdentifier(searchSurfaceChromeID)
        chrome.translatesAutoresizingMaskIntoConstraints = false
        chrome.wantsLayer = true
        chrome.layer?.cornerRadius = LauncherChromeTokens.searchBarCornerRadius
        chrome.layer?.cornerCurve = .continuous
        chrome.layer?.borderWidth = 0.6
        chrome.layer?.borderColor = ColorTokens.searchSurfaceBorder.cgColor
        chrome.layer?.backgroundColor = ColorTokens.searchSurfaceFill.cgColor
        chrome.layer?.shadowColor = NSColor.black.cgColor
        chrome.layer?.shadowOpacity = ColorTokens.searchSurfaceShadowOpacity
        chrome.layer?.shadowRadius = 8
        chrome.layer?.shadowOffset = CGSize(width: 0, height: -1)
        view.addSubview(chrome, positioned: .below, relativeTo: nil)
        NSLayoutConstraint.activate([
            chrome.topAnchor.constraint(equalTo: view.topAnchor),
            chrome.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            chrome.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            chrome.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
    }

    static func configureDetailTable(_ tableView: NSTableView, rowHeight: CGFloat = LauncherChromeTokens.detailTableRowHeight) {
        tableView.style = .plain
        tableView.rowHeight = rowHeight
        tableView.backgroundColor = .clear
        tableView.selectionHighlightStyle = .regular
        tableView.usesAlternatingRowBackgroundColors = false
        tableView.intercellSpacing = NSSize(width: 0, height: LauncherChromeTokens.detailTableRowSpacing)
        tableView.gridStyleMask = []
        styleDetailTableColumns(tableView)
    }

    static func styleDetailTableColumns(_ tableView: NSTableView) {
        for column in tableView.tableColumns {
            let headerCell = column.headerCell
            headerCell.font = TypographyTokens.caption2(weight: .semibold)
            headerCell.textColor = .labelColor.withAlphaComponent(0.82)
        }
    }

    /// Standard vertical list scroll chrome for module detail tables, outlines, and stacks.
    static func configureVerticalListScroll(_ scrollView: NSScrollView, horizontal: Bool = false) {
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = horizontal
        scrollView.autohidesScrollers = true
        scrollView.clipsToBounds = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
    }

    /// Keeps scroll document frames aligned with visible rows/content so scrollers appear when content overflows.
    static func syncVerticalListDocumentFrame(in scrollView: NSScrollView) {
        guard let documentView = scrollView.documentView else { return }
        let width = max(scrollView.contentSize.width, 1)
        let viewportHeight = scrollView.contentSize.height
        let contentHeight = verticalListContentHeight(for: documentView)
        let height = max(contentHeight, viewportHeight)
        let frame = NSRect(x: 0, y: 0, width: width, height: height)
        guard documentView.frame != frame else { return }
        documentView.frame = frame
        scrollView.reflectScrolledClipView(scrollView.contentView)
    }

    /// Wires a detail table/outline scroll view and resyncs the document frame when the clip view resizes.
    /// AppKit posts frame notifications off the Swift MainActor; hop before touching UI state.
    static func wireVerticalListScroll(
        _ scrollView: NSScrollView,
        documentView: NSView,
        onClipViewResize: @escaping @MainActor () -> Void
    ) {
        scrollView.documentView = documentView
        if let outlineView = documentView as? NSOutlineView {
            outlineView.autoresizingMask = [.width]
        } else if let tableView = documentView as? NSTableView {
            tableView.autoresizingMask = [.width]
        }
        configureVerticalListScroll(scrollView)
        scrollView.contentView.postsBoundsChangedNotifications = true
        scrollView.clipResizeObserver = ScrollClipResizeObserver(
            scrollView: scrollView,
            handler: onClipViewResize
        )
    }

    private static func verticalListContentHeight(for documentView: NSView) -> CGFloat {
        if let outlineView = documentView as? NSOutlineView {
            let rowCount = outlineView.numberOfRows
            return rowCount > 0 ? outlineView.rect(ofRow: rowCount - 1).maxY : 0
        }
        if let tableView = documentView as? NSTableView {
            let rowCount = tableView.numberOfRows
            return rowCount > 0 ? tableView.rect(ofRow: rowCount - 1).maxY : 0
        }
        if let textView = documentView as? NSTextView,
           let layoutManager = textView.layoutManager,
           let textContainer = textView.textContainer {
            layoutManager.ensureLayout(for: textContainer)
            let used = layoutManager.usedRect(for: textContainer)
            return used.height + textView.textContainerInset.height * 2
        }
        documentView.layoutSubtreeIfNeeded()
        return max(documentView.fittingSize.height, documentView.frame.height)
    }

    /// Pins a vertical stack used as an `NSScrollView` document view for Auto Layout content sizing.
    static func pinVerticalStackDocumentView(_ stack: NSStackView, in scrollView: NSScrollView) {
        stack.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: scrollView.contentView.topAnchor),
            stack.leadingAnchor.constraint(equalTo: scrollView.contentView.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: scrollView.contentView.trailingAnchor),
            stack.widthAnchor.constraint(equalTo: scrollView.contentView.widthAnchor)
        ])
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
    nonisolated func geekLayoutAccentLayers() {
        guard let sublayers = layer?.sublayers else { return }
        for sublayer in sublayers where sublayer.name == "geekAccentStrip" {
            GeekUIKit.layoutAccentStrip(sublayer, in: bounds)
        }
        for sublayer in sublayers where sublayer.name == "geekSidebarAccent" {
            GeekUIKit.layoutSidebarAccent(sublayer, in: bounds)
        }
    }
}

/// AppKit-only segmented control that avoids `NSSegmentedControl`.
/// macOS 26 routes `NSSegmentedControl` sizing/layout through DesignLibrary SwiftUI, and AppKit may
/// call those Objective-C entry points on the main thread without Swift's MainActor executor.
/// Keep this as a self-drawing `NSControl` so launcher show/hide never enters that crash-prone path.
final class LauncherSegmentedControl: NSControl {
    nonisolated(unsafe) var trackingMode: NSSegmentedControl.SwitchTracking = .selectOne
    private nonisolated(unsafe) var cachedIntrinsicSize = NSSize(width: 80, height: 28)
    private nonisolated(unsafe) var labels: [String] = []
    private nonisolated(unsafe) var selectedIndex: Int = -1

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    nonisolated var segmentCount: Int {
        get { labels.count }
        set {
            let count = max(0, newValue)
            if labels.count < count {
                labels.append(contentsOf: Array(repeating: "", count: count - labels.count))
            } else if labels.count > count {
                labels.removeLast(labels.count - count)
            }
            if selectedIndex >= count {
                selectedIndex = count - 1
            }
            refreshCachedIntrinsicSize()
            needsDisplay = true
        }
    }

    nonisolated var selectedSegment: Int {
        get { selectedIndex }
        set {
            selectedIndex = labels.indices.contains(newValue) ? newValue : -1
            needsDisplay = true
        }
    }

    nonisolated func setLabel(_ label: String, forSegment segment: Int) {
        guard labels.indices.contains(segment) else { return }
        labels[segment] = label
        refreshCachedIntrinsicSize()
        needsDisplay = true
    }

    nonisolated func label(forSegment segment: Int) -> String? {
        guard labels.indices.contains(segment) else { return nil }
        return labels[segment]
    }

    nonisolated func refreshCachedIntrinsicSize() {
        let font = font ?? NSFont.systemFont(ofSize: NSFont.smallSystemFontSize)
        var width: CGFloat = 8
        for title in labels {
            width += ceil((title as NSString).size(withAttributes: [.font: font]).width) + 20
        }
        cachedIntrinsicSize = NSSize(width: max(width, 80), height: 28)
        invalidateIntrinsicContentSize()
    }

    nonisolated override var intrinsicContentSize: NSSize {
        cachedIntrinsicSize
    }

    nonisolated override func mouseDown(with event: NSEvent) {
        guard !labels.isEmpty else { return }
        let point = convert(event.locationInWindow, from: nil)
        guard bounds.contains(point) else { return }
        let index = min(max(Int(point.x / max(bounds.width / CGFloat(labels.count), 1)), 0), labels.count - 1)
        selectedSegment = index
        Task { @MainActor [weak self] in
            guard let self, let action = self.action else { return }
            NSApp.sendAction(action, to: self.target, from: self)
        }
    }

    nonisolated override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard !labels.isEmpty else { return }

        let rect = bounds.insetBy(dx: 0.5, dy: 0.5)
        let outer = NSBezierPath(roundedRect: rect, xRadius: 7, yRadius: 7)
        NSColor.controlBackgroundColor.withAlphaComponent(0.75).setFill()
        outer.fill()
        NSColor.separatorColor.withAlphaComponent(0.45).setStroke()
        outer.lineWidth = 1
        outer.stroke()

        let segmentWidth = bounds.width / CGFloat(labels.count)
        let font = font ?? NSFont.systemFont(ofSize: NSFont.smallSystemFontSize, weight: .medium)
        for (index, title) in labels.enumerated() {
            let segmentRect = NSRect(
                x: bounds.minX + CGFloat(index) * segmentWidth,
                y: bounds.minY,
                width: segmentWidth,
                height: bounds.height
            ).insetBy(dx: 2, dy: 2)

            if index == selectedIndex {
                NSColor.controlAccentColor.withAlphaComponent(0.18).setFill()
                NSBezierPath(roundedRect: segmentRect, xRadius: 5, yRadius: 5).fill()
            }

            if index > 0 {
                NSColor.separatorColor.withAlphaComponent(0.35).setStroke()
                let divider = NSBezierPath()
                divider.move(to: NSPoint(x: segmentRect.minX - 2, y: bounds.minY + 5))
                divider.line(to: NSPoint(x: segmentRect.minX - 2, y: bounds.maxY - 5))
                divider.lineWidth = 1
                divider.stroke()
            }

            let color: NSColor = index == selectedIndex ? .controlAccentColor : .secondaryLabelColor
            let attributes: [NSAttributedString.Key: Any] = [
                .font: font,
                .foregroundColor: color
            ]
            let size = (title as NSString).size(withAttributes: attributes)
            let drawRect = NSRect(
                x: segmentRect.midX - size.width / 2,
                y: segmentRect.midY - size.height / 2,
                width: size.width,
                height: size.height
            )
            (title as NSString).draw(in: drawRect, withAttributes: attributes)
        }
    }
}

// AppKit display cycle calls layout() without Swift MainActor executor — do not isolate this view.
final class GeekGlassPanel: NSView {
    @MainActor
    init(accentHex: String?) {
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        GeekUIKit.configureContentSurface(self)
        if let accentHex, let chrome = GeekUIKit.contentSurfaceChrome(in: self) {
            _ = GeekUIKit.installAccentStrip(on: chrome, color: ColorTokens.color(hex: accentHex))
        }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    nonisolated override func layout() {
        super.layout()
        GeekUIKit.contentSurfaceChrome(in: self)?.geekLayoutAccentLayers()
    }
}
