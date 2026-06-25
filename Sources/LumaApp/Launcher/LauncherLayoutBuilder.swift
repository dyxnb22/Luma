import AppKit
import LumaCore

@MainActor
enum LauncherLayoutBuilder {
    private static let margin = LauncherChromeTokens.contentMargin

    static func install(
        on root: NSView,
        searchBar: LumaSearchBar,
        commandHintBar: CommandHintBar,
        listView: LauncherListView,
        hintBar: LauncherHintBar,
        actionPanel: LauncherActionPanel,
        contentContainer: NSView,
        detailContainer: NSView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        closeDetailTarget: AnyObject,
        closeDetailAction: Selector
    ) {
        searchBar.translatesAutoresizingMaskIntoConstraints = false
        commandHintBar.translatesAutoresizingMaskIntoConstraints = false
        listView.translatesAutoresizingMaskIntoConstraints = false
        hintBar.translatesAutoresizingMaskIntoConstraints = false
        actionPanel.translatesAutoresizingMaskIntoConstraints = false
        contentContainer.translatesAutoresizingMaskIntoConstraints = false

        contentContainer.addSubview(listView)
        root.addSubview(searchBar)
        root.addSubview(commandHintBar)
        root.addSubview(contentContainer)
        root.addSubview(hintBar)
        root.addSubview(actionPanel)

        NSLayoutConstraint.activate([
            searchBar.topAnchor.constraint(equalTo: root.topAnchor, constant: margin),
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

            listView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            listView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            listView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            listView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            hintBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin),
            hintBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -margin),
            hintBar.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -14),

            actionPanel.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: margin + 4),
            actionPanel.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -(margin + 4)),
            actionPanel.bottomAnchor.constraint(equalTo: hintBar.topAnchor, constant: -8)
        ])

        installDetailContainer(
            detailContainer: detailContainer,
            detailTopBar: detailTopBar,
            detailTitleLabel: detailTitleLabel,
            contentContainer: contentContainer,
            closeDetailTarget: closeDetailTarget,
            closeDetailAction: closeDetailAction
        )
    }

    private static func installDetailContainer(
        detailContainer: NSView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        contentContainer: NSView,
        closeDetailTarget: AnyObject,
        closeDetailAction: Selector
    ) {
        detailContainer.translatesAutoresizingMaskIntoConstraints = false
        detailContainer.alphaValue = 0
        detailContainer.isHidden = true
        detailTopBar.translatesAutoresizingMaskIntoConstraints = false
        detailTopBar.wantsLayer = true

        let backButton = NSButton(title: "", target: closeDetailTarget, action: closeDetailAction)
        GeekUIKit.styleDetailBackButton(backButton)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        detailTitleLabel.font = TypographyTokens.callout
        detailTitleLabel.textColor = .labelColor
        detailTitleLabel.isEditable = false
        detailTitleLabel.isBordered = false
        detailTitleLabel.drawsBackground = false
        detailTitleLabel.alignment = .center
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
        contentContainer.addSubview(detailContainer)

        NSLayoutConstraint.activate([
            detailContainer.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            detailContainer.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            detailContainer.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            detailContainer.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            detailTopBar.topAnchor.constraint(equalTo: detailContainer.topAnchor),
            detailTopBar.leadingAnchor.constraint(equalTo: detailContainer.leadingAnchor),
            detailTopBar.trailingAnchor.constraint(equalTo: detailContainer.trailingAnchor),
            detailTopBar.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.detailTopBarHeight),

            backButton.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor, constant: 4),
            backButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            detailTitleLabel.centerXAnchor.constraint(equalTo: detailTopBar.centerXAnchor),
            detailTitleLabel.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            closeButton.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor, constant: -4),
            closeButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            separator.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: detailTopBar.bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 1)
        ])
    }
}
