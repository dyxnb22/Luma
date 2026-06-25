import AppKit

@MainActor
enum LauncherLayoutBuilder {
    static func install(
        on root: NSView,
        searchBar: LumaSearchBar,
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
        listView.translatesAutoresizingMaskIntoConstraints = false
        hintBar.translatesAutoresizingMaskIntoConstraints = false
        actionPanel.translatesAutoresizingMaskIntoConstraints = false
        contentContainer.translatesAutoresizingMaskIntoConstraints = false

        contentContainer.addSubview(listView)
        root.addSubview(searchBar)
        root.addSubview(contentContainer)
        root.addSubview(hintBar)
        root.addSubview(actionPanel)

        NSLayoutConstraint.activate([
            searchBar.topAnchor.constraint(equalTo: root.topAnchor, constant: 16),
            searchBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 16),
            searchBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -16),
            searchBar.heightAnchor.constraint(equalToConstant: 52),

            contentContainer.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 8),
            contentContainer.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 16),
            contentContainer.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -16),
            contentContainer.bottomAnchor.constraint(equalTo: hintBar.topAnchor, constant: -4),

            listView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            listView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            listView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            listView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            hintBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 16),
            hintBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -16),
            hintBar.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -12),

            actionPanel.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 24),
            actionPanel.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -24),
            actionPanel.bottomAnchor.constraint(equalTo: hintBar.topAnchor, constant: -6)
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
        backButton.image = NSImage(systemSymbolName: "chevron.left", accessibilityDescription: nil)
        backButton.imagePosition = .imageLeading
        backButton.title = "Back"
        backButton.bezelStyle = .regularSquare
        backButton.isBordered = false
        backButton.font = .systemFont(ofSize: 13, weight: .medium)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        detailTitleLabel.font = .systemFont(ofSize: 15, weight: .semibold)
        detailTitleLabel.textColor = .labelColor
        detailTitleLabel.isEditable = false
        detailTitleLabel.isBordered = false
        detailTitleLabel.drawsBackground = false
        detailTitleLabel.alignment = .center
        detailTitleLabel.translatesAutoresizingMaskIntoConstraints = false

        let closeButton = NSButton(title: "", target: closeDetailTarget, action: closeDetailAction)
        closeButton.image = NSImage(systemSymbolName: "xmark", accessibilityDescription: nil)
        closeButton.bezelStyle = .regularSquare
        closeButton.isBordered = false
        closeButton.translatesAutoresizingMaskIntoConstraints = false

        let separator = NSBox()
        separator.boxType = .separator
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
            detailTopBar.heightAnchor.constraint(equalToConstant: 40),

            backButton.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor, constant: 16),
            backButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            detailTitleLabel.centerXAnchor.constraint(equalTo: detailTopBar.centerXAnchor),
            detailTitleLabel.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            closeButton.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor, constant: -16),
            closeButton.centerYAnchor.constraint(equalTo: detailTopBar.centerYAnchor),

            separator.leadingAnchor.constraint(equalTo: detailTopBar.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: detailTopBar.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: detailTopBar.bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 1)
        ])
    }
}
