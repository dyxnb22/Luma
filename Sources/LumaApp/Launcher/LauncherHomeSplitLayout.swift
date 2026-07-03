import AppKit
import LumaCore

/// Toggles empty-query home between full-width list and Open Apps + command guide columns.
@MainActor
final class LauncherHomeSplitLayout {
    let guidePane: LauncherHomeGuidePane

    private let listView: LauncherListView
    private let divider: NSBox
    private let listFullWidthTrailing: NSLayoutConstraint
    private let listSplitWidth: NSLayoutConstraint
    private var isActive = false

    static func install(
        in contentContainer: NSView,
        listView: LauncherListView
    ) -> LauncherHomeSplitLayout {
        let guidePane = LauncherHomeGuidePane()
        let divider = NSBox()
        GeekUIKit.configureDetailSeparator(divider)
        divider.translatesAutoresizingMaskIntoConstraints = false
        divider.isHidden = true

        contentContainer.addSubview(divider)
        contentContainer.addSubview(guidePane)

        let listSplitWidth = listView.widthAnchor.constraint(
            equalToConstant: LauncherChromeTokens.homeLeftColumnWidth
        )
        listSplitWidth.priority = .required
        listSplitWidth.isActive = false

        let listFullWidthTrailing = listView.trailingAnchor.constraint(
            equalTo: contentContainer.trailingAnchor,
            constant: -8
        )

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
            guidePane: guidePane,
            divider: divider,
            listFullWidthTrailing: listFullWidthTrailing,
            listSplitWidth: listSplitWidth
        )
    }

    private init(
        listView: LauncherListView,
        guidePane: LauncherHomeGuidePane,
        divider: NSBox,
        listFullWidthTrailing: NSLayoutConstraint,
        listSplitWidth: NSLayoutConstraint
    ) {
        self.listView = listView
        self.guidePane = guidePane
        self.divider = divider
        self.listFullWidthTrailing = listFullWidthTrailing
        self.listSplitWidth = listSplitWidth
    }

    func setActive(_ active: Bool) {
        guard active != isActive else { return }
        isActive = active
        guidePane.isHidden = !active
        divider.isHidden = !active
        listSplitWidth.isActive = active
        listFullWidthTrailing.isActive = !active
        listView.setCompactHomeColumn(active)
    }

    var active: Bool { isActive }
}
