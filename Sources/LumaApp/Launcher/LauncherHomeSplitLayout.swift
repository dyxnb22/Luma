import AppKit
import LumaCore

/// Toggles column split: Open Apps (left) + guide or module detail (right). ADR-032.
@MainActor
final class LauncherHomeSplitLayout {
    let guidePane: LauncherHomeGuidePane

    private let listView: LauncherListView
    private let detailContainer: LauncherOverlayHostView
    private let divider: NSBox
    private let listFullWidthTrailing: NSLayoutConstraint
    private let listSplitWidth: NSLayoutConstraint
    private let detailRightColumnConstraints: [NSLayoutConstraint]
    private weak var contentContainer: NSView?
    private var columnSplitActive = false
    private var rightPane: LauncherSplitRightPane = .hidden

    static func install(
        in contentContainer: NSView,
        listView: LauncherListView,
        detailContainer: LauncherOverlayHostView
    ) -> LauncherHomeSplitLayout {
        let guidePane = LauncherHomeGuidePane()
        let divider = NSBox()
        GeekUIKit.configureDetailSeparator(divider)
        divider.translatesAutoresizingMaskIntoConstraints = false
        divider.isHidden = true

        detailContainer.translatesAutoresizingMaskIntoConstraints = false
        contentContainer.addSubview(divider)
        contentContainer.addSubview(guidePane)
        contentContainer.addSubview(detailContainer)

        let listSplitWidth = listView.widthAnchor.constraint(
            equalToConstant: LauncherChromeTokens.homeLeftColumnWidth
        )
        listSplitWidth.priority = .required
        listSplitWidth.isActive = false

        let listFullWidthTrailing = listView.trailingAnchor.constraint(
            equalTo: contentContainer.trailingAnchor,
            constant: -8
        )

        let detailRightColumnConstraints = [
            detailContainer.topAnchor.constraint(equalTo: contentContainer.topAnchor, constant: 8),
            detailContainer.leadingAnchor.constraint(
                equalTo: divider.trailingAnchor,
                constant: LauncherChromeTokens.homeSplitColumnGap
            ),
            detailContainer.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor, constant: -8),
            detailContainer.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor, constant: -8)
        ]
        detailRightColumnConstraints.forEach { $0.isActive = false }

        NSLayoutConstraint.activate([
            divider.leadingAnchor.constraint(
                equalTo: listView.trailingAnchor,
                constant: LauncherChromeTokens.homeSplitColumnGap
            ),
            divider.widthAnchor.constraint(equalToConstant: LauncherChromeTokens.homeSplitDividerWidth),
            divider.topAnchor.constraint(equalTo: contentContainer.topAnchor, constant: 8),
            divider.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor, constant: -8),

            guidePane.leadingAnchor.constraint(
                equalTo: divider.trailingAnchor,
                constant: LauncherChromeTokens.homeSplitColumnGap
            ),
            guidePane.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor, constant: -8),
            guidePane.topAnchor.constraint(equalTo: contentContainer.topAnchor, constant: 8),
            guidePane.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor, constant: -8)
        ])

        let layout = LauncherHomeSplitLayout(
            listView: listView,
            detailContainer: detailContainer,
            guidePane: guidePane,
            divider: divider,
            listFullWidthTrailing: listFullWidthTrailing,
            listSplitWidth: listSplitWidth,
            detailRightColumnConstraints: detailRightColumnConstraints,
            contentContainer: contentContainer
        )
        layout.activateDefaultListWidthConstraints()
        return layout
    }

    private init(
        listView: LauncherListView,
        detailContainer: LauncherOverlayHostView,
        guidePane: LauncherHomeGuidePane,
        divider: NSBox,
        listFullWidthTrailing: NSLayoutConstraint,
        listSplitWidth: NSLayoutConstraint,
        detailRightColumnConstraints: [NSLayoutConstraint],
        contentContainer: NSView
    ) {
        self.listView = listView
        self.detailContainer = detailContainer
        self.guidePane = guidePane
        self.divider = divider
        self.listFullWidthTrailing = listFullWidthTrailing
        self.listSplitWidth = listSplitWidth
        self.detailRightColumnConstraints = detailRightColumnConstraints
        self.contentContainer = contentContainer
    }

    /// Single-column results layout — list spans the content container.
    func activateDefaultListWidthConstraints() {
        listSplitWidth.isActive = false
        listFullWidthTrailing.isActive = true
    }

    func setColumnSplitActive(_ active: Bool) {
        guard active != columnSplitActive else { return }
        columnSplitActive = active
        let flags = LauncherHomeSplitConstraintPolicy.flags(for: currentState)
        divider.isHidden = !flags.dividerVisible
        listSplitWidth.isActive = flags.listSplitWidthActive
        listFullWidthTrailing.isActive = flags.listFullWidthTrailingActive
        listView.setCompactHomeColumn(active)
        if active {
            GeekUIKit.installHomeListColumnSurface(on: listView)
        }
        if active, let contentContainer {
            contentContainer.addSubview(divider, positioned: .above, relativeTo: listView)
            contentContainer.addSubview(guidePane, positioned: .above, relativeTo: listView)
            contentContainer.addSubview(detailContainer, positioned: .above, relativeTo: listView)
        }
        applyRightPaneLayout()
    }

    func setRightPane(_ pane: LauncherSplitRightPane) {
        guard pane != rightPane else { return }
        rightPane = pane
        applyRightPaneLayout()
    }

    /// Cross-fades the right column from module detail to the home command guide (ADR-032).
    func crossfadeFromDetailToGuide(
        commands: [CommandDefinition],
        completion: @escaping @MainActor () -> Void
    ) {
        rightPane = .guide
        guidePane.applyCatalog(commands)
        guidePane.isHidden = false
        guidePane.alphaValue = 0
        guidePane.passesHitTests = false

        detailContainer.isHidden = false
        detailContainer.alphaValue = 1
        detailContainer.passesHitTests = false
        detailRightColumnConstraints.forEach { $0.isActive = columnSplitActive }

        NSAnimationContext.runAnimationGroup { context in
            context.duration = MotionTokens.detailPaneCrossfadeDuration
            context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
            detailContainer.animator().alphaValue = 0
            guidePane.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor in
                guard let self else { return }
                self.detailContainer.alphaValue = 0
                self.detailContainer.isHidden = true
                self.detailContainer.passesHitTests = false
                self.guidePane.alphaValue = 1
                self.guidePane.passesHitTests = true
                completion()
            }
        }
    }

    /// Cross-fades the right column from the home command guide into module detail (ADR-032).
    func crossfadeFromGuideToDetail(
        prepareDetail: @escaping @MainActor () -> Void,
        completion: @escaping @MainActor () -> Void
    ) {
        rightPane = .detail
        prepareDetail()

        guidePane.isHidden = false
        guidePane.alphaValue = 1
        guidePane.passesHitTests = false

        detailContainer.isHidden = false
        detailContainer.alphaValue = 0
        detailContainer.passesHitTests = false
        detailRightColumnConstraints.forEach { $0.isActive = columnSplitActive }

        NSAnimationContext.runAnimationGroup { context in
            context.duration = MotionTokens.detailPaneCrossfadeDuration
            context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
            guidePane.animator().alphaValue = 0
            detailContainer.animator().alphaValue = 1
        } completionHandler: { [weak self] in
            Task { @MainActor in
                guard let self else { return }
                self.guidePane.isHidden = true
                self.guidePane.alphaValue = 1
                self.guidePane.passesHitTests = true
                self.detailContainer.alphaValue = 1
                self.detailContainer.passesHitTests = true
                completion()
            }
        }
    }

    private func applyRightPaneLayout() {
        let flags = LauncherHomeSplitConstraintPolicy.flags(for: currentState)
        guidePane.isHidden = !flags.guideVisible
        detailContainer.isHidden = !flags.detailVisible
        detailRightColumnConstraints.forEach { $0.isActive = flags.detailRightColumnActive }
    }

    private var currentState: LauncherHomeSplitState {
        LauncherHomeSplitState(columnSplitActive: columnSplitActive, rightPane: rightPane)
    }
}
