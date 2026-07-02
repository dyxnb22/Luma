import AppKit

/// Base window that restores standard text-editing shortcuts without an Edit menu.
@MainActor
class LumaWindow: NSWindow {
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if LumaStandardEditShortcuts.performKeyEquivalent(event, in: self) {
            return true
        }
        if styleMask.contains(.closable),
           event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "w" {
            close()
            return true
        }
        return super.performKeyEquivalent(with: event)
    }
}
