import AppKit
import LumaCore

enum LauncherSplitRightPane: Equatable {
    case guide
    case detail
    case hidden
}

/// Toggles empty-query home between full-width list and Open Apps + right pane (guide or module detail).
@MainActor
final class LauncherHomeSplitLayout {
    let guidePane: LauncherHomeGuidePane

    private let listView: LauncherListView
    private let detailContainer: LauncherOverlayHostView
    private let divider: NSBox
    private let listFullWidthTrailing: NSLayoutConstraint
    private let listSplitWidth: NSLayoutConstraint
    private let detailFullBleedConstraints: [NSLayoutConstraint]
    private let detailRightColumnConstraints: [NSLayoutConstraint]
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

        let detailFullBleedConstraints = [
            detailContainer.topAnchor.constraint(equalTo: contentContainer.topAnchor),
            detailContainer.leadingAnchor.constraint(equalTo: contentContainer.leadingAnchor),
            detailContainer.trailingAnchor.constraint(equalTo: contentContainer.trailingAnchor),
            detailContainer.bottomAnchor.constraint(equalTo: contentContainer.bottomAnchor)
        ]
        detailFullBleedConstraints.forEach { $0.isActive = false }

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

        return LauncherHomeSplitLayout(
            listView: listView,
            detailContainer: detailContainer,
            guidePane: guidePane,
            divider: divider,
            listFullWidthTrailing: listFullWidthTrailing,
            listSplitWidth: listSplitWidth,
            detailFullBleedConstraints: detailFullBleedConstraints,
            detailRightColumnConstraints: detailRightColumnConstraints
        )
    }

    private init(
        listView: LauncherListView,
        detailContainer: LauncherOverlayHostView,
        guidePane: LauncherHomeGuidePane,
        divider: NSBox,
        listFullWidthTrailing: NSLayoutConstraint,
        listSplitWidth: NSLayoutConstraint,
        detailFullBleedConstraints: [NSLayoutConstraint],
        detailRightColumnConstraints: [NSLayoutConstraint]
    ) {
        self.listView = listView
        self.detailContainer = detailContainer
        self.guidePane = guidePane
        self.divider = divider
        self.listFullWidthTrailing = listFullWidthTrailing
        self.listSplitWidth = listSplitWidth
        self.detailFullBleedConstraints = detailFullBleedConstraints
        self.detailRightColumnConstraints = detailRightColumnConstraints
    }

    func setColumnSplitActive(_ active: Bool) {
        guard active != columnSplitActive else { return }
        columnSplitActive = active
        divider.isHidden = !active
        listSplitWidth.isActive = active
        listFullWidthTrailing.isActive = !active
        listView.setCompactHomeColumn(active)
        applyRightPaneLayout()
    }

    func setRightPane(_ pane: LauncherSplitRightPane) {
        guard pane != rightPane else { return }
        rightPane = pane
        applyRightPaneLayout()
    }

    var isColumnSplitActive: Bool { columnSplitActive }

    private func applyRightPaneLayout() {
        guidePane.isHidden = !(columnSplitActive && rightPane == .guide)
        let showsDetail = rightPane == .detail
        detailContainer.isHidden = !showsDetail

        let useRightColumnDetail = columnSplitActive && showsDetail
        let useFullBleedDetail = !columnSplitActive && showsDetail

        detailRightColumnConstraints.forEach { $0.isActive = useRightColumnDetail }
        detailFullBleedConstraints.forEach { $0.isActive = useFullBleedDetail }
    }
}
