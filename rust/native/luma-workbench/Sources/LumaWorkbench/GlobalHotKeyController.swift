import AppKit
import Carbon.HIToolbox
import LumaWorkbenchCore

/// Registers the global activation shortcut.
///
/// Carbon's `RegisterEventHotKey` is used on purpose: it needs no Accessibility permission, unlike
/// a `CGEventTap`. The host asks for no TCC permission at all just to be summoned.
final class GlobalHotKeyController {
    enum RegistrationError: Error, CustomStringConvertible {
        case handlerInstallFailed(OSStatus)
        case registrationFailed(OSStatus)

        var description: String {
            switch self {
            case .handlerInstallFailed(let status):
                return "Could not install the hotkey event handler (OSStatus \(status))."
            case .registrationFailed(let status):
                return "Could not register the global hotkey (OSStatus \(status)). "
                    + "Another application is probably already using it."
            }
        }
    }

    let definition: HotKeyDefinition

    private let onActivate: () -> Void
    private var debouncer = HotKeyDebouncer()
    private var hotKeyRef: EventHotKeyRef?
    private var eventHandlerRef: EventHandlerRef?

    init(definition: HotKeyDefinition = .defaultActivation, onActivate: @escaping () -> Void) {
        self.definition = definition
        self.onActivate = onActivate
    }

    deinit {
        unregister()
    }

    func register() throws {
        guard hotKeyRef == nil else { return }

        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )
        let context = Unmanaged.passUnretained(self).toOpaque()
        let installStatus = InstallEventHandler(
            GetApplicationEventTarget(),
            hotKeyEventHandler,
            1,
            &eventType,
            context,
            &eventHandlerRef
        )
        guard installStatus == noErr else {
            throw RegistrationError.handlerInstallFailed(installStatus)
        }

        let hotKeyID = EventHotKeyID(signature: HotKeyDefinition.signature, id: 1)
        let status = RegisterEventHotKey(
            definition.keyCode,
            definition.carbonModifiers,
            hotKeyID,
            GetApplicationEventTarget(),
            0,
            &hotKeyRef
        )
        guard status == noErr, hotKeyRef != nil else {
            removeEventHandler()
            throw RegistrationError.registrationFailed(status)
        }
    }

    func unregister() {
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
            self.hotKeyRef = nil
        }
        removeEventHandler()
    }

    private func removeEventHandler() {
        if let eventHandlerRef {
            RemoveEventHandler(eventHandlerRef)
            self.eventHandlerRef = nil
        }
    }

    fileprivate func handleHotKeyPressed() {
        guard debouncer.shouldAccept(at: ProcessInfo.processInfo.systemUptime) else { return }
        onActivate()
    }
}

/// Carbon calls back through a plain C function pointer, so the controller travels as `userData`.
private let hotKeyEventHandler: EventHandlerUPP = { _, event, userData in
    guard let userData, let event else { return OSStatus(eventNotHandledErr) }
    var hotKeyID = EventHotKeyID()
    let status = GetEventParameter(
        event,
        EventParamName(kEventParamDirectObject),
        EventParamType(typeEventHotKeyID),
        nil,
        MemoryLayout<EventHotKeyID>.size,
        nil,
        &hotKeyID
    )
    guard status == noErr, hotKeyID.signature == HotKeyDefinition.signature else {
        return OSStatus(eventNotHandledErr)
    }
    Unmanaged<GlobalHotKeyController>.fromOpaque(userData)
        .takeUnretainedValue()
        .handleHotKeyPressed()
    return noErr
}
