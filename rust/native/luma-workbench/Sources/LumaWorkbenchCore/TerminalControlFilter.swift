import Foundation

/// Drops terminal string controls before they reach SwiftTerm's parser.
///
/// Luma's product UI is emitted with CSI sequences, so it does not need these
/// OSC/APC/DCS/PM/SOS channels. Refusing them prevents a terminal child (or a
/// remote SSH peer) from reading/writing the macOS pasteboard through OSC 52
/// and keeps unterminated strings out of SwiftTerm's parser buffers.
public struct TerminalControlFilter: Sendable {
    private enum State: Sendable {
        case passthrough
        case escape
        case discardingString(afterEscape: Bool, bellTerminates: Bool)
    }

    private var state: State = .passthrough
    /// Number of UTF-8 continuation bytes expected after a leading byte. C1 OSC/APC values
    /// (0x9D / 0x9F) overlap the UTF-8 continuation range, so they are control bytes only when
    /// they occur outside a UTF-8 scalar.
    private var utf8ContinuationBytes = 0

    public init() {}

    /// Starts filtering a new PTY stream with no parser state inherited from the old child.
    ///
    /// An exited child may leave an OSC/APC string or UTF-8 scalar unterminated. Those bytes
    /// belong only to that PTY session and must not suppress or reinterpret the next child's
    /// first output.
    public mutating func reset() {
        self = TerminalControlFilter()
    }

    /// Filters one PTY chunk while preserving state across chunk boundaries.
    public mutating func filter(_ input: ArraySlice<UInt8>) -> [UInt8] {
        var output: [UInt8] = []
        output.reserveCapacity(input.count)

        for byte in input {
            if case .passthrough = state, utf8ContinuationBytes > 0 {
                if byte & 0xc0 == 0x80 {
                    output.append(byte)
                    utf8ContinuationBytes -= 1
                    continue
                }
                // Invalid or interrupted UTF-8 must not make the next byte look protected.
                utf8ContinuationBytes = 0
            }

            switch state {
            case .passthrough:
                if byte == 0x1b {
                    state = .escape
                } else if byte == 0x9d { // 8-bit OSC
                    state = .discardingString(afterEscape: false, bellTerminates: true)
                } else if matchesNonOscStringControl(byte) { // 8-bit DCS/SOS/PM/APC
                    state = .discardingString(afterEscape: false, bellTerminates: false)
                } else if byte == 0x9c { // Stray 8-bit ST is a control, not printable UTF-8.
                    continue
                } else {
                    appendPlaintextByte(byte, to: &output)
                }

            case .escape:
                switch byte {
                case 0x5d: // ESC ] (OSC)
                    state = .discardingString(afterEscape: false, bellTerminates: true)
                case 0x50, 0x58, 0x5e, 0x5f: // ESC P/X/^/_ (DCS/SOS/PM/APC)
                    state = .discardingString(afterEscape: false, bellTerminates: false)
                default:
                    output.append(0x1b)
                    appendPlaintextByte(byte, to: &output)
                    state = .passthrough
                }

            case .discardingString(let afterEscape, let bellTerminates):
                switch byte {
                case 0x18, 0x1a, 0x9c: // CAN, SUB, and 8-bit ST terminate a string.
                    state = .passthrough
                case 0x07 where bellTerminates: // BEL terminates OSC only.
                    state = .passthrough
                case 0x5c where afterEscape: // ESC \\ (ST)
                    state = .passthrough
                case 0x1b:
                    state = .discardingString(
                        afterEscape: true,
                        bellTerminates: bellTerminates
                    )
                default:
                    state = .discardingString(
                        afterEscape: false,
                        bellTerminates: bellTerminates
                    )
                }
            }
        }

        return output
    }

    private mutating func appendPlaintextByte(_ byte: UInt8, to output: inout [UInt8]) {
        output.append(byte)
        utf8ContinuationBytes = switch byte {
        case 0xc2...0xdf: 1
        case 0xe0...0xef: 2
        case 0xf0...0xf4: 3
        default: 0
        }
    }

    private func matchesNonOscStringControl(_ byte: UInt8) -> Bool {
        switch byte {
        case 0x90, 0x98, 0x9e, 0x9f: true
        default: false
        }
    }
}
