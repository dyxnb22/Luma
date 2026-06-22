import Carbon.HIToolbox
import Foundation

/// Carbon delivers hot-key events on the main thread, but not through Swift's MainActor.
/// Keep this handler free of @MainActor closures so Swift 6 executor checks do not crash.
private func lumaHotKeyEventHandler(
    _: EventHandlerCallRef?,
    _: EventRef?,
    userData: UnsafeMutableRawPointer?
) -> OSStatus {
    guard let userData else { return noErr }
    Unmanaged<HotkeyController>.fromOpaque(userData).takeUnretainedValue().schedulePress()
    return noErr
}

@MainActor
final class HotkeyController {
    nonisolated(unsafe) private var hotKeyRef: EventHotKeyRef?
    nonisolated(unsafe) private var handlerRef: EventHandlerRef?
    private let onPress: @MainActor () -> Void

    init(onPress: @escaping @MainActor () -> Void) throws {
        self.onPress = onPress
        try installHandler()
    }

    func register(_ combo: KeyCombo) throws {
        unregister()

        var ref: EventHotKeyRef?
        let hotkeyID = EventHotKeyID(signature: Self.signature, id: 1)
        let status = RegisterEventHotKey(
            combo.virtualKeyCode,
            combo.carbonModifiers,
            hotkeyID,
            GetEventDispatcherTarget(),
            0,
            &ref
        )

        guard status == noErr else {
            throw HotkeyError.registrationFailed(status)
        }

        hotKeyRef = ref
    }

    func unregister() {
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
            self.hotKeyRef = nil
        }
    }

    func simulatePressForDevelopment() {
        onPress()
    }

    nonisolated func schedulePress() {
        let raw = Unmanaged.passUnretained(self).toOpaque()
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                Unmanaged<HotkeyController>.fromOpaque(raw).takeUnretainedValue().onPress()
            }
        }
    }

    private func installHandler() throws {
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )
        let userData = Unmanaged.passUnretained(self).toOpaque()
        let status = InstallEventHandler(
            GetEventDispatcherTarget(),
            lumaHotKeyEventHandler,
            1,
            &eventType,
            userData,
            &handlerRef
        )

        guard status == noErr else {
            throw HotkeyError.handlerInstallFailed(status)
        }
    }

    deinit {
        if let handlerRef {
            RemoveEventHandler(handlerRef)
        }
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
        }
    }

    private static let signature: OSType = {
        let scalars = Array("LUMA".unicodeScalars).map(\.value)
        return (scalars[0] << 24) | (scalars[1] << 16) | (scalars[2] << 8) | scalars[3]
    }()
}
