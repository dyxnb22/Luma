import Foundation
import CoreGraphics

public struct ActionID: Hashable, Sendable, Codable {
    public let module: ModuleIdentifier
    public let key: String

    public init(module: ModuleIdentifier, key: String) {
        self.module = module
        self.key = key
    }
}

public struct Action: Sendable, Hashable, Codable {
    public let id: ActionID
    public let title: String
    public let kind: ActionKind
    public let runsOn: ExecutionContext
    public let confirmation: Confirmation

    public init(
        id: ActionID,
        title: String,
        kind: ActionKind,
        runsOn: ExecutionContext = .background,
        confirmation: Confirmation = .none
    ) {
        self.id = id
        self.title = title
        self.kind = kind
        self.runsOn = runsOn
        self.confirmation = confirmation
    }
}

public enum ActionKind: Sendable, Hashable, Codable {
    case launchApp(URL)
    case focusWindow(windowID: UInt32, pid: Int32, title: String, bounds: WindowBounds?)
    case copyToPasteboard(String)
    case openURL(URL)
    case revealInFinder(URL)
    case insertText(String)
    case applyWindowLayout(String)
    case translateText(String)
    case custom(payload: Data, handler: ModuleIdentifier)
    case noop
}

public struct WindowBounds: Sendable, Hashable, Codable {
    public let x: CGFloat
    public let y: CGFloat
    public let width: CGFloat
    public let height: CGFloat

    public init(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
    }

    public init(_ rect: CGRect) {
        x = rect.origin.x
        y = rect.origin.y
        width = rect.size.width
        height = rect.size.height
    }

    public var rect: CGRect {
        CGRect(x: x, y: y, width: width, height: height)
    }
}

public enum ExecutionContext: Sendable, Hashable, Codable {
    case main
    case background
}

public enum Confirmation: Sendable, Hashable, Codable {
    case none
    case requireReturn
    case requireSecondModifier
}
