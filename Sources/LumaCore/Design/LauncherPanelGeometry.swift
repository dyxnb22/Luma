import CoreGraphics
import Foundation

/// Pure panel sizing and placement math for the borderless launcher (`fullSizeContentView`).
/// Borderless launcher frames match content size; do not apply `anchorPoint` transforms on the root view.
@MainActor
public enum LauncherPanelGeometry {
    public static let horizontalScreenInset: CGFloat = 48
    public static let verticalScreenInset: CGFloat = 96

    /// Content size that fits within a screen's visible frame, honoring `LauncherChromeTokens` clamps.
    public static func contentSize(for visibleFrame: CGRect) -> CGSize {
        let availableWidth = visibleFrame.width - horizontalScreenInset
        let availableHeight = visibleFrame.height - verticalScreenInset

        let targetWidth = min(
            availableWidth,
            min(
                LauncherChromeTokens.maxPanelWidth,
                max(LauncherChromeTokens.minPanelWidth, LauncherChromeTokens.defaultPanelWidth)
            )
        )
        let targetHeight = min(
            availableHeight,
            min(
                LauncherChromeTokens.maxPanelHeight,
                max(LauncherChromeTokens.minPanelHeight, LauncherChromeTokens.defaultPanelHeight)
            )
        )

        let width: CGFloat
        if availableWidth < LauncherChromeTokens.minPanelWidth {
            width = max(320, availableWidth)
        } else if targetWidth < LauncherChromeTokens.defaultPanelWidth {
            width = max(
                LauncherChromeTokens.minPanelWidth,
                min(targetWidth, visibleFrame.width * LauncherChromeTokens.panelWidthScreenRatio)
            )
        } else {
            width = targetWidth
        }

        let height: CGFloat
        if availableHeight < LauncherChromeTokens.minPanelHeight {
            height = max(400, availableHeight)
        } else if targetHeight < LauncherChromeTokens.defaultPanelHeight {
            height = max(
                LauncherChromeTokens.minPanelHeight,
                min(targetHeight, visibleFrame.height * LauncherChromeTokens.panelHeightScreenRatio)
            )
        } else {
            height = targetHeight
        }

        return CGSize(width: width, height: height)
    }

    /// Horizontally centered frame with upper-third vertical bias inside `visibleFrame`.
    public static func panelFrame(
        fitting visibleFrame: CGRect,
        verticalBias: CGFloat = LauncherChromeTokens.panelVerticalBias
    ) -> CGRect {
        let size = contentSize(for: visibleFrame)
        let origin = CGPoint(
            x: round(visibleFrame.midX - size.width / 2),
            y: round(visibleFrame.minY + (visibleFrame.height - size.height) * verticalBias)
        )
        return CGRect(origin: origin, size: size)
    }

    /// Centers an arbitrary window frame inside a visible screen rect (Settings, sheets).
    public static func centeredOrigin(
        for windowSize: CGSize,
        in visibleFrame: CGRect
    ) -> CGPoint {
        CGPoint(
            x: round(visibleFrame.midX - windowSize.width / 2),
            y: round(visibleFrame.midY - windowSize.height / 2)
        )
    }
}
