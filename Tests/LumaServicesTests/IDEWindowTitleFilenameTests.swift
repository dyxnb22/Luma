import Testing
@testable import LumaServices

@Suite struct IDEWindowTitleFilenameTests {
  private let cursorBundle = "com.todesktop.230313mzl4w4u92"

  @Test func extractsFilenameFromCursorTitle() {
    let name = IDEWindowTitle.filename(
      rawTitle: "README.md — Luma — Cursor",
      bundleID: cursorBundle,
      appName: "Cursor"
    )
    #expect(name == "README.md")
  }

  @Test func nonIDEReturnsNil() {
    let name = IDEWindowTitle.filename(
      rawTitle: "Inbox — Google Chrome",
      bundleID: "com.google.Chrome",
      appName: "Google Chrome"
    )
    #expect(name == nil)
  }
}
