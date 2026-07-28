import Darwin
import Foundation

// Swift imports both the C `flock` function and `struct flock` under the same name on macOS 26,
// which makes a qualified call resolve to the structure initializer. Bind the POSIX function to
// an unambiguous Swift name.
@_silgen_name("flock")
private func systemFlock(_ descriptor: Int32, _ operation: Int32) -> Int32

public enum SingleInstanceLockError: Error, Equatable, CustomStringConvertible {
    case couldNotOpen(path: String, errno: Int32)
    case couldNotLock(path: String, errno: Int32)

    public var description: String {
        switch self {
        case .couldNotOpen(let path, let code):
            return "could not open the Luma session lock at \(path) (errno \(code))"
        case .couldNotLock(let path, let code):
            return "could not lock the Luma session at \(path) (errno \(code))"
        }
    }
}

/// A process-scoped lock for the one long-running workbench session.
///
/// `flock` is released by the kernel if the host crashes. The descriptor is close-on-exec so the
/// bundled Rust TUI cannot accidentally keep the host lock alive after the host has exited.
public final class SingleInstanceLock {
    private var descriptor: Int32?

    private init(descriptor: Int32) {
        self.descriptor = descriptor
    }

    /// Returns `nil` when another process already owns the lock.
    public static func acquire(at url: URL) throws -> SingleInstanceLock? {
        let path = url.path
        let descriptor = Darwin.open(path, O_CREAT | O_RDWR | O_CLOEXEC, S_IRUSR | S_IWUSR)
        guard descriptor >= 0 else {
            throw SingleInstanceLockError.couldNotOpen(path: path, errno: errno)
        }

        guard systemFlock(descriptor, LOCK_EX | LOCK_NB) == 0 else {
            let code = errno
            _ = Darwin.close(descriptor)
            if code == EWOULDBLOCK {
                return nil
            }
            throw SingleInstanceLockError.couldNotLock(path: path, errno: code)
        }

        return SingleInstanceLock(descriptor: descriptor)
    }

    deinit {
        release()
    }

    func release() {
        guard let descriptor else { return }
        self.descriptor = nil
        _ = systemFlock(descriptor, LOCK_UN)
        _ = Darwin.close(descriptor)
    }
}
