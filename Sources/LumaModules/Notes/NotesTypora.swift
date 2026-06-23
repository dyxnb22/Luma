import AppKit

public enum NotesTypora {
    public static func open(_ url: URL) {
        NSWorkspace.shared.open(url)
    }
}
