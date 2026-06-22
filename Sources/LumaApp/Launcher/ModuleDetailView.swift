import AppKit
import LumaCore

@MainActor
protocol ModuleDetailView: AnyObject {
    var detailView: NSView { get }
    var moduleTitle: String { get }
    func activate()
    func deactivate()
}

@MainActor
final class TranslateDetailView: ModuleDetailView {
    let moduleTitle = "Translate"
    let detailView: NSView = {
        let v = NSView()
        let label = NSTextField(wrappingLabelWithString: "Type to translate. Use prefix \"translate \" or \"tr \".")
        label.font = .systemFont(ofSize: 14)
        label.textColor = .secondaryLabelColor
        label.translatesAutoresizingMaskIntoConstraints = false
        v.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: v.topAnchor, constant: 20),
            label.leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -16)
        ])
        return v
    }()
    func activate() {}
    func deactivate() {}
}

@MainActor
final class ClipboardDetailView: ModuleDetailView {
    let moduleTitle = "Clipboard"
    let detailView: NSView = {
        let v = NSView()
        let label = NSTextField(wrappingLabelWithString: "Type to search clipboard history. Prefix \"clip \".")
        label.font = .systemFont(ofSize: 14)
        label.textColor = .secondaryLabelColor
        label.translatesAutoresizingMaskIntoConstraints = false
        v.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: v.topAnchor, constant: 20),
            label.leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -16)
        ])
        return v
    }()
    func activate() {}
    func deactivate() {}
}

@MainActor
final class CalculatorDetailView: ModuleDetailView {
    let moduleTitle = "Calculator"
    let detailView: NSView = {
        let v = NSView()
        let label = NSTextField(wrappingLabelWithString: "Type = followed by an expression. Example: =3*7+1")
        label.font = .systemFont(ofSize: 14)
        label.textColor = .secondaryLabelColor
        label.translatesAutoresizingMaskIntoConstraints = false
        v.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: v.topAnchor, constant: 20),
            label.leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -16)
        ])
        return v
    }()
    func activate() {}
    func deactivate() {}
}

@MainActor
final class WindowsDetailView: ModuleDetailView {
    let moduleTitle = "Windows"
    let detailView: NSView = {
        let v = NSView()
        let label = NSTextField(wrappingLabelWithString: "Type to focus a window by title or app name.")
        label.font = .systemFont(ofSize: 14)
        label.textColor = .secondaryLabelColor
        label.translatesAutoresizingMaskIntoConstraints = false
        v.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: v.topAnchor, constant: 20),
            label.leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: 16),
            label.trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -16)
        ])
        return v
    }()
    func activate() {}
    func deactivate() {}
}

@MainActor
enum ModuleDetailRegistry {
    static func make(for id: ModuleIdentifier) -> (any ModuleDetailView)? {
        switch id {
        case .translate:   return TranslateDetailView()
        case .clipboard:   return ClipboardDetailView()
        case .calculator:  return CalculatorDetailView()
        case .windows:     return WindowsDetailView()
        default:           return nil
        }
    }
}
