import CoreGraphics
import Foundation

/// Launcher panel geometry, spacing, and chrome values shared by home and detail surfaces.
@MainActor
public enum LauncherChromeTokens {
    // Panel
    public static let panelCornerRadius: CGFloat = 22
    public static let panelBorderWidth: CGFloat = 0.75
    public static let panelBorderAlpha: CGFloat = 0.34
    public static let panelSheenTopAlpha: CGFloat = 0.28
    public static let panelSheenMidAlpha: CGFloat = 0.10
    public static let panelDepthBottomAlpha: CGFloat = 0.06

    public static let defaultPanelWidth: CGFloat = 940
    public static let defaultPanelHeight: CGFloat = 760
    public static let minPanelWidth: CGFloat = 720
    public static let maxPanelWidth: CGFloat = 980
    public static let minPanelHeight: CGFloat = 640
    public static let maxPanelHeight: CGFloat = 820

    /// Fixed width for Open Apps column on empty-query home (right pane shows command guide).
    public static let homeLeftColumnWidth: CGFloat = 360
    public static let homeSplitDividerWidth: CGFloat = 1
    public static let homeSplitColumnGap: CGFloat = 12
    /// Fallback when the screen is smaller than `defaultPanel*`.
    public static let panelWidthScreenRatio: CGFloat = 0.44
    public static let panelHeightScreenRatio: CGFloat = 0.58
    /// Vertical placement bias (0 = bottom of screen, 1 = top). Spotlight sits in the upper third.
    public static let panelVerticalBias: CGFloat = 0.68

    // Layout
    public static let contentMargin: CGFloat = 20
    public static let searchBarHeight: CGFloat = 56
    public static let searchBarCornerRadius: CGFloat = 14
    public static let searchBarInsetH: CGFloat = 14
    public static let searchBarIconSize: CGFloat = 18
    public static let searchFontSize: CGFloat = 20

    public static let commandHintHeight: CGFloat = 50
    public static let hintBarHeight: CGFloat = 30
    public static let contentTopGap: CGFloat = 8
    public static let contentBottomGap: CGFloat = 6

    // Performance strip (header status line above search)
    public static let performanceStripHeight: CGFloat = 20
    public static let performanceStripGap: CGFloat = 6
    public static let performanceMetricGap: CGFloat = 12
    public static let performanceMetricCornerRadius: CGFloat = 6
    public static let performanceWarningCPU: Double = 75
    public static let performanceWarningMemoryRatio: Double = 0.85

    // List
    public static let listRowSpacing: CGFloat = 5
    public static let listRowCornerRadius: CGFloat = 10
    public static let listRowHeight: CGFloat = 46
    public static let listRowHeightNested: CGFloat = 40
    public static let listRowIconSize: CGFloat = 34
    public static let listRowIconSizeNested: CGFloat = 22
    public static let listRowSelectionAlpha: CGFloat = 0.16
    public static let listRowHoverAlpha: CGFloat = 0.06
    public static let listRowSelectedShadowRadius: CGFloat = 6
    public static let homeListSurfaceCornerRadius: CGFloat = 14
    public static let sectionHeaderHeight: CGFloat = 28

    // Detail
    public static let detailMargin: CGFloat = 20
    public static let detailTopBarHeight: CGFloat = 44
    public static let detailToolbarHeight: CGFloat = 40
    public static let detailToolbarTallHeight: CGFloat = 64
    public static let detailFooterHeight: CGFloat = 28
    public static let detailSectionGap: CGFloat = 10
    public static let detailSurfaceCornerRadius: CGFloat = 12
    public static let detailSurfaceBorderAlpha: CGFloat = 0.28
    public static let detailTableRowHeight: CGFloat = 40
    public static let detailTableRowSpacing: CGFloat = 4
    public static let detailTableRowCornerRadius: CGFloat = 8
    public static let detailTableRowPaddingH: CGFloat = 12
}
