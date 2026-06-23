import AppKit

@MainActor
enum LauncherLayoutBuilder {
    static func install(
        on root: NSView,
        searchBar: LumaSearchBar,
        sidebarContainer: NSView,
        sidebarHeader: NSTextField,
        sidebarScrollView: NSScrollView,
        sidebarStack: FlippedStackView,
        sidebarSeparator: NSView,
        contentContainer: NSView,
        homeScrollView: NSScrollView,
        featureGridView: FeatureFlowView,
        resultsScrollView: NSScrollView,
        resultsStackView: FlippedStackView,
        loadingLabel: NSTextField,
        detailContainer: NSView,
        detailTopBar: NSView,
        detailTitleLabel: NSTextField,
        closeDetailTarget: AnyObject,
        closeDetailAction: Selector
    ) {
        searchBar.translatesAutoresizingMaskIntoConstraints = false
        sidebarHeader.font = .systemFont(ofSize: 11, weight: .semibold)
        sidebarHeader.textColor = .secondaryLabelColor
        sidebarHeader.translatesAutoresizingMaskIntoConstraints = false

        sidebarStack.orientation = .vertical
        sidebarStack.spacing = 2
        sidebarStack.alignment = .leading
        sidebarStack.translatesAutoresizingMaskIntoConstraints = false

        sidebarSeparator.wantsLayer = true
        sidebarSeparator.layer?.backgroundColor = NSColor.separatorColor.withAlphaComponent(0.25).cgColor
        sidebarSeparator.translatesAutoresizingMaskIntoConstraints = false

        sidebarContainer.translatesAutoresizingMaskIntoConstraints = false
        sidebarContainer.addSubview(sidebarHeader)

        sidebarScrollView.hasVerticalScroller = true
        sidebarScrollView.hasHorizontalScroller = false
        sidebarScrollView.autohidesScrollers = true
        sidebarScrollView.drawsBackground = false
        sidebarScrollView.borderType = .noBorder
        sidebarScrollView.translatesAutoresizingMaskIntoConstraints = false
        sidebarScrollView.documentView = sidebarStack

        sidebarContainer.addSubview(sidebarScrollView)
        sidebarContainer.addSubview(sidebarSeparator)

        contentContainer.translatesAutoresizingMaskIntoConstraints = false
        homeScrollView.hasVerticalScroller = true
        homeScrollView.hasHorizontalScroller = false
        homeScrollView.drawsBackground = false
        homeScrollView.borderType = .noBorder
        homeScrollView.translatesAutoresizingMaskIntoConstraints = false
        homeScrollView.documentView = featureGridView

        resultsStackView.orientation = .vertical
        resultsStackView.spacing = 6
        resultsStackView.alignment = .leading
        resultsStackView.translatesAutoresizingMaskIntoConstraints = false

        resultsScrollView.hasVerticalScroller = true
        resultsScrollView.drawsBackground = false
        resultsScrollView.borderType = .noBorder
        resultsScrollView.translatesAutoresizingMaskIntoConstraints = false
        resultsScrollView.documentView = resultsStackView

        loadingLabel.font = .systemFont(ofSize: 13)
        loadingLabel.textColor = .secondaryLabelColor
        loadingLabel.alignment = .center
        loadingLabel.isHidden = true
        loadingLabel.translatesAutoresizingMaskIntoConstraints = false

        contentContainer.addSubview(homeScrollView)
        contentContainer.addSubview(resultsScrollView)
        contentContainer.addSubview(loadingLabel)

        root.addSubview(searchBar)
        root.addSubview(sidebarContainer)
        root.addSubview(contentContainer)

        NSLayoutConstraint.activate([
            searchBar.topAnchor.constraint(equalTo: root.topAnchor, constant: 20),
            searchBar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 20),
            searchBar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -20),

            sidebarContainer.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 12),
            sidebarContainer.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 20),
            sidebarContainer.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -20),
            sidebarContainer.widthAnchor.constraint(equalToConstant: 180),

            sidebarHeader.topAnchor.constraint(equalTo: sidebarContainer.topAnchor, constant: 4),
            sidebarHeader.leadingAnchor.constraint(equalTo: sidebarContainer.leadingAnchor, constant: 4),
            sidebarHeader.trailingAnchor.constraint(equalTo: sidebarSeparator.leadingAnchor, constant: -8),

            sidebarScrollView.topAnchor.constraint(equalTo: sidebarHeader.bottomAnchor, constant: 4),
            sidebarScrollView.leadingAnchor.constraint(equalTo: sidebarContainer.leadingAnchor),
            sidebarScrollView.trailingAnchor.constraint(equalTo: sidebarSeparator.leadingAnchor),
            sidebarScrollView.bottomAnchor.constraint(equalTo: sidebarContainer.bottomAnchor),

            sidebarStack.topAnchor.constraint(equalTo: sidebarScrollView.contentView.topAnchor),
            sidebarStack.leadingAnchor.constraint(equalTo: sidebarScrollView.contentView.leadingAnchor),
            sidebarStack.trailingAnchor.constraint(equalTo: sidebarScrollView.contentView.trailingAnchor),
            sidebarStack.bottomAnchor.constraint(equalTo: sidebarScrollView.contentView.bottomAnchor),
            sidebarStack.widthAnchor.constraint(equalTo: sidebarScrollView.contentView.widthAnchor),

            sidebarSeparator.topAnchor.constraint(equalTo: sidebarContainer.topAnchor),
            sidebarSeparator.trailingAnchor.constraint(equalTo: sidebarContainer.trailingAnchor),
            sidebarSeparator.bottomAnchor.constraint(equalTo: sidebarContainer.bottomAnchor),
            sidebarSeparator.widthAnchor.constraint(equalToConstant: 1),

            contentContainer.topAnchor.constraint(equalTo: searchBar.bottomAnchor, constant: 12),
            contentContainer.leadingAnchor.constraint(equalTo: sidebarContainer.trailingAnchor),
            contentContainer.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -20),
            contentContainer.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -20),

            homeScrollView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            homeScrollView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor, constant: 16),
            homeScrollView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            homeScrollView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            resultsScrollView.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            resultsScrollView.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor, constant: 16),
            resultsScrollView.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            resultsScrollView.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor),

            resultsStackView.topAnchor.constraint(equalTo: resultsScrollView.contentView.topAnchor),
            resultsStackView.leadingAnchor.constraint(equalTo: resultsScrollView.contentView.leadingAnchor),
            resultsStackView.trailingAnchor.constraint(equalTo: resultsScrollView.contentView.trailingAnchor),
            resultsStackView.widthAnchor.constraint(equalTo: resultsScrollView.contentView.widthAnchor),

            loadingLabel.centerXAnchor.constraint(equalTo: contentContainer.centerXAnchor),
            loadingLabel.centerYAnchor.constraint(equalTo: contentContainer.centerYAnchor)
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
