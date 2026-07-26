import Foundation

/// Drops OSC and APC strings before they reach SwiftTerm's parser.
///
/// Luma's product UI is emitted with CSI sequences, so it does not need these
/// string-style control channels. Refusing them prevents a terminal child (or a
/// remote SSH peer) from reading/writing the macOS pasteboard through OSC 52.
/// It also means an unterminated OSC/APC stream is discarded as it arrives
/// instead of accumulating in SwiftTerm's parser buffers.
public struct TerminalControlFilter: Sendable {
    private enum State: Sendable {
        case passthrough
        case escape
        case discardingString(afterEscape: Bool)
    }

    private var state: State = .passthrough

    public init() {}

    /// Filters one PTY chunk while preserving state across chunk boundaries.
    public mutating func filter(_ input: ArraySlice<UInt8>) -> [UInt8] {
        var output: [UInt8] = []
        output.reserveCapacity(input.count)

        for byte in input {
            switch state {
            case .passthrough:
                if byte == 0x1b {
                    state = .escape
                } else if byte == 0x9d || byte == 0x9f { // 8-bit OSC/APC
                    state = .discardingString(afterEscape: false)
                } else {
                    output.append(byte)
                }

            case .escape:
                switch byte {
                case 0x5d, 0x5f: // ESC ] (OSC), ESC _ (APC)
                    state = .discardingString(afterEscape: false)
                default:
                    output.append(0x1b)
                    output.append(byte)
                    state = .passthrough
                }

            case .discardingString(let afterEscape):
                switch byte {
                case 0x07, 0x18, 0x1a, 0x9c: // BEL, CAN, SUB, 8-bit ST terminate a string.
                    state = .passthrough
                case 0x5c where afterEscape: // ESC \\ (ST)
                    state = .passthrough
                case 0x1b:
                    state = .discardingString(afterEscape: true)
                default:
                    state = .discardingString(afterEscape: false)
                }
            }
        }

        return output
    }
}
