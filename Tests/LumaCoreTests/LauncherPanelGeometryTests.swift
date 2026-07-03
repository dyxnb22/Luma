import CoreGraphics
import Testing
import LumaCore

@MainActor
@Test func panelGeometryDefaultSizeOnLargeDisplay() {
    let visible = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    let size = LauncherPanelGeometry.contentSize(for: visible)
    #expect(size.width == LauncherChromeTokens.defaultPanelWidth)
    #expect(size.height == LauncherChromeTokens.defaultPanelHeight)
}

@MainActor
@Test func panelGeometryCentersHorizontally() {
    let visible = CGRect(x: 100, y: 50, width: 1600, height: 900)
    let frame = LauncherPanelGeometry.panelFrame(fitting: visible)
    #expect(frame.midX == visible.midX)
}

@MainActor
@Test func panelGeometryUpperThirdBias() {
    let visible = CGRect(x: 0, y: 0, width: 1440, height: 900)
    let frame = LauncherPanelGeometry.panelFrame(fitting: visible, verticalBias: 0.68)
    let expectedY = round(visible.minY + (visible.height - frame.height) * 0.68)
    #expect(frame.origin.y == expectedY)
}

@MainActor
@Test func panelGeometryUsesDefaultWidthWhenAvailableEqualsDefault() {
    // visible width 988 → available 940 → must use full default, not ratio-clamp to 720
    let visible = CGRect(x: 0, y: 0, width: 988, height: 900)
    let size = LauncherPanelGeometry.contentSize(for: visible)
    #expect(size.width == LauncherChromeTokens.defaultPanelWidth)
    #expect(size.height == LauncherChromeTokens.defaultPanelHeight)
}

@MainActor
@Test func panelGeometryClampsOnSmallDisplay() {
    let visible = CGRect(x: 0, y: 0, width: 700, height: 500)
    let size = LauncherPanelGeometry.contentSize(for: visible)
    #expect(size.width <= visible.width)
    #expect(size.height <= visible.height)
}

@MainActor
@Test func panelGeometryCenteredOriginForAuxiliaryWindow() {
    let visible = CGRect(x: 200, y: 100, width: 1200, height: 800)
    let windowSize = CGSize(width: 720, height: 520)
    let origin = LauncherPanelGeometry.centeredOrigin(for: windowSize, in: visible)
    #expect(origin.x + windowSize.width / 2 == visible.midX)
    #expect(origin.y + windowSize.height / 2 == visible.midY)
}
