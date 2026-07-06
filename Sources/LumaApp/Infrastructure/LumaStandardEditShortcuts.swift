@preconcurrency import AppKit

/// Standard text-editing shortcuts for Luma windows.
/// Menu-bar apps have no Edit menu, so AppKit's default `performKeyEquivalent`
/// chain never reaches `selectAll:` / `copy:` / `paste:` / etc. Route them here.
enum LumaStandardEditShortcuts {
    nonisolated static func performKeyEquivalent(_ event: NSEvent, in window: NSWindow) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        guard flags.contains(.command) else { return false }
        guard let key = event.charactersIgnoringModifiers?.lowercased() else { return false }

        let hasShift = flags.contains(.shift)
        let hasOption = flags.contains(.option)
        let hasControl = flags.contains(.control)

        if hasShift, !hasOption, !hasControl, key == "z" {
            return redo(in: window)
        }

        guard !hasShift, !hasOption, !hasControl else { return false }

        switch key {
        case "a":
            return selectAll(in: window)
        case "c":
            return copy(in: window)
        case "v":
            return paste(in: window)
        case "x":
            return cut(in: window)
        case "z":
            return undo(in: window)
        case "y":
            return redo(in: window)
        default:
            return false
        }
    }

    /// Fallback for code paths that receive `keyDown` instead of `performKeyEquivalent`.
    nonisolated static func handleKeyDown(_ event: NSEvent, in window: NSWindow?) -> Bool {
        guard let window else { return false }
        return performKeyEquivalent(event, in: window)
    }

    @discardableResult nonisolated static func selectAll(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return selectAll(for: responder)
    }

    @discardableResult nonisolated static func copy(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return copy(for: responder, in: window)
    }

    @discardableResult nonisolated static func paste(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return paste(for: responder, in: window)
    }

    @discardableResult nonisolated static func cut(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return cut(for: responder, in: window)
    }

    @discardableResult nonisolated static func undo(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return undo(for: responder)
    }

    @discardableResult nonisolated static func redo(in window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return false }
        return redo(for: responder)
    }

    @discardableResult nonisolated static func selectAll(for responder: NSResponder) -> Bool {
        if let textView = responder as? NSTextView, textView.isSelectable {
            textView.selectAll(nil)
            return true
        }
        if let textField = responder as? NSTextField, textField.isEditable || textField.isSelectable {
            textField.selectText(nil)
            return true
        }
        return false
    }

    @discardableResult nonisolated static func copy(for responder: NSResponder, in window: NSWindow) -> Bool {
        if let textView = responder as? NSTextView {
            guard textView.isSelectable else { return false }
            guard textView.selectedRange().length > 0 else { return false }
            textView.copy(nil)
            return true
        }
        if let textField = responder as? NSTextField, textField.isEditable || textField.isSelectable {
            if let editor = activeEditor(for: textField, in: window),
               editor.selectedRange().length > 0 {
                editor.copy(nil)
                return true
            }
            let value = textField.stringValue
            guard !value.isEmpty else { return false }
            writePasteboard(value)
            return true
        }
        return false
    }

    @discardableResult nonisolated static func paste(for responder: NSResponder, in window: NSWindow) -> Bool {
        guard hasPasteableString else { return false }
        if let textView = responder as? NSTextView {
            guard textView.isEditable else { return false }
            textView.paste(nil)
            return true
        }
        if let textField = responder as? NSTextField {
            guard textField.isEditable else { return false }
            guard let editor = activeEditor(for: textField, in: window) else { return false }
            editor.paste(nil)
            syncTextField(textField, from: editor)
            return true
        }
        return false
    }

    @discardableResult nonisolated static func cut(for responder: NSResponder, in window: NSWindow) -> Bool {
        if let textView = responder as? NSTextView {
            guard textView.isEditable, textView.isSelectable else { return false }
            guard textView.selectedRange().length > 0 else { return false }
            textView.cut(nil)
            return true
        }
        if let textField = responder as? NSTextField {
            guard textField.isEditable else { return false }
            guard let editor = activeEditor(for: textField, in: window) else { return false }
            guard editor.selectedRange().length > 0 else { return false }
            editor.cut(nil)
            syncTextField(textField, from: editor)
            return true
        }
        return false
    }

    @discardableResult nonisolated static func undo(for responder: NSResponder) -> Bool {
        if let textView = responder as? NSTextView {
            guard textView.isEditable, textView.allowsUndo else { return false }
            guard let undoManager = textView.undoManager, undoManager.canUndo else { return false }
            undoManager.undo()
            return true
        }
        return false
    }

    @discardableResult nonisolated static func redo(for responder: NSResponder) -> Bool {
        if let textView = responder as? NSTextView {
            guard textView.isEditable, textView.allowsUndo else { return false }
            guard let undoManager = textView.undoManager, undoManager.canRedo else { return false }
            undoManager.redo()
            return true
        }
        return false
    }

    nonisolated private static func activeEditor(for textField: NSTextField, in window: NSWindow) -> NSTextView? {
        if let editor = textField.currentEditor() as? NSTextView {
            return editor
        }
        window.makeFirstResponder(textField)
        textField.selectText(nil)
        return window.fieldEditor(false, for: textField) as? NSTextView
    }

    nonisolated private static func syncTextField(_ textField: NSTextField, from editor: NSTextView) {
        let updated = editor.string
        guard textField.stringValue != updated else { return }
        textField.stringValue = updated
        NotificationCenter.default.post(
            name: NSControl.textDidChangeNotification,
            object: textField
        )
    }

    nonisolated private static var hasPasteableString: Bool {
        NSPasteboard.general.string(forType: .string) != nil
    }

    nonisolated private static func writePasteboard(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
