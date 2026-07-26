import Foundation

/// Stable identifiers for the local `Luma.app` bundle. Kept in one place so the build script,
/// the window frame autosave, and any diagnostics text cannot drift apart.
public enum HostIdentity {
    public static let bundleIdentifier = "com.luma.next.workbench"
    public static let applicationName = "Luma"

    /// AppKit frame autosave key. Changing it forgets the stored frame once, nothing else.
    public static let windowFrameAutosaveName = "LumaWorkbenchWindow"
}
