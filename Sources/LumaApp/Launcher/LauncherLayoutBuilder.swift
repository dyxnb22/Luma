import AppKit
import LumaCore

@MainActor
enum LauncherLayoutBuilder {
    private static let margin = LauncherChromeTokens.contentMargin

    static func install(
        on root: NSView,
        performanceStrip: LauncherPerformanceStripView,
        searchBar: LumaSearchBar,
        commandHintBar: CommandHintBar,
        listView: LauncherListView,
        hintBar: LauncherHintBar,
        actionPanel: LauncherActionPanel,
        contentContainer: NSView,
        detailContainer: LauncherOverlayHostView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        closeDetailTarget: AnyObject,
        closeDetailAction: Selector,
        onPanelSpacingChanged: (() -> Void)? = nil
    ) {
        performanceStrip.translatesAutoresizingMaskIntoConstraints = false
        searchBar.translatesAutoresizingMaskIntoConstraints = false
        commandHintBar.translatesAutoresizingMaskIntoConstraints = false
        listView.translatesAutoresizingMaskIntoConstraints = false
        hintBar.translatesAutoresizingMaskIntoConstraints = false
        actionPanel.translatesAutoresizingMaskIntoConstraints = false
        contentContainer.translatesAutoresizingMaskIntoConstraints = false
        contentContainer.clipsToBounds = true
        GeekUIKit.installHomeListSurface(on: contentContainer)

        contentContainer.addSubview(listView)
        root.addSubview(performanceStrip)
        root.addSubview(searchBar)
        root.addSubview(commandHintBar)
        root.addSubview(contentContainer)
        root.addSubview(hintBar)
        root.addSubview(actionPanel)

        let searchBarTopGap = searchBar.topAnchor.constraint(
            equalTo: performanceStrip.bottomAnchor,
            constant: LauncherChromeTokens.performanceStripGap
        )
        performanceStrip.onPresenceChanged = { visible in
            searchBarTopGap.constant = visible ? LauncherChromeTokens.performanceStripGap : 0
            onPanelSpacingChanged?()
        }

        NSLayoutConstraint.activate([
            performanceStrip.topAnchor.constraint(equalTo: root.topAnchor, constant: margin),
            performanceStrip.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            performanceStrip.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),

            searchBarTopGap,
            searchBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            searchBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),
            searchBar.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.searchBarHeight),

            commandHintBar.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 4),
            commandHintBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            commandHintBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),

            contentContainer.topAnchor.constraint(equalTo: commandHintBar.bottomAnchor, constant: LauncherChromeTokens.contentTopGap),
            contentContainer.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            contentContainer.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),
            contentContainer.bottomAnchor.constraint(equalTo: hintBar.topAnchor, constant: -LauncherChromeTokens.contentBottomGap),

            listView.topAnchor.constraint(equalTo: contentContainer.topAnchor, constant: 8),
            listView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor, constant: 8),
            listView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor, constant: -8),

            hintBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            hintBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),
            hintBar.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -14),

            actionPanel.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin + 4),
            actionPanel.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -(margin + 4))
        ])
        actionPanel.configureLayout(in: root, fallbackBottomAnchor: hintBar)

        installDetailContainer(
            detailContainer: detailContainer,
            detailTopBar: detailTopBar,
            detailTitleLabel: detailTitleLabel,
            closeDetailTarget: closeDetailTarget,
            closeDetailAction: closeDetailAction
        )
    }

    private static func installDetailContainer(
        detailContainer: LauncherOverlayHostView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        closeDetailTarget: AnyObject,
        closeDetailAction: Selector
    ) {
        detailContainer.translatesAutoresizingMaskIntoConstraints = false
        detailContainer.alphaValue = 0
        detailContainer.isHidden = true
        detailContainer.clipsToBounds = true
        detailTopBar.translatesAutoresizingMaskIntoConstraints = false
        detailTopBar.clipsToBounds = true

        let backButton = NSButton(title: "", target: closeDetailTarget, action: closeDetailAction)
        GeekUIKit.styleDetailBackButton(backButton)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        detailTitleLabel.font = TypographyTokens.callout
        detailTitleLabel.textColor = .labelColor
        detailTitleLabel.isEditable = false
        detailTitleLabel.isBordered = false
        detailTitleLabel.drawsBackground = false
        detailTitleLabel.alignment = .center
        detailTitleLabel.lineBreakMode = .byTruncatingTail
        detailTitleLabel.maximumNumberOfLines = 1
        detailTitleLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        detailTitleLabel.translatesAutoresizingMaskIntoConstraints = false

        let closeButton = NSButton(title: "", target: closeDetailTarget, action: closeDetailAction)
        GeekUIKit.styleDetailCloseButton(closeButton)
        closeButton.translatesAutoresizingMaskIntoConstraints = false

        let separator = NSBox()
        GeekUIKit.configureDetailSeparator(separator)
        separator.translatesAutoresizingMaskIntoConstraints = false

        detailTopBar.addSubview(backButton)
        detailTopBar.addSubview(detailTitleLabel)
        detailTopBar.addSubview(closeButton)
        detailTopBar.addSubview(separator)
        detailContainer.addSubview(detailTopBar)

        NSLayoutConstraint.activate([
            detailTopBar.topAnchor.constraint(equalTo: detailContainer.topAnchor),
            detailTopBar.leadingAnchor.constraint(equalTo: detailContainer.leadingAnchor),
            detailTopBar.trailingAnchor.constraint(equalTo: detailContainer.trailingAnchor),
            detailTopBar.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.detailTopBarHeight),

            backButton.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor, constant: 4),
            backButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            detailTitleLabel.centerXAnchor.constraint(equalTo: detailTopBar.centerXAnchor),
            detailTitleLabel.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),
            detailTitleLabel.leadingAnchor.constraint(greaterThanOrEqualTo: backButton.trailingAnchor, constant: 8),
            detailTitleLabel.trailingAnchor.constraint(lessThanOrEqualTo: closeButton.leadingAnchor, constant: -8),

            closeButton.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor, constant: -4),
            closeButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            separator.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: detailTopBar.bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 1)
        ])
    }
}
