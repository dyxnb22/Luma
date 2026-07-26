import Foundation

/// Resolution of the `luma` binary that ships inside `Luma.app`.
///
/// The host deliberately never consults `PATH`: a GUI-launched app would otherwise pick up
/// whichever build happens to be first in the developer's shell environment.
public enum BundledExecutable {
    /// Name of the CLI as it is copied into `Contents/MacOS/`.
    public static let name = "luma"

    /// Argument used to open the interactive workbench.
    public static let tuiArguments = ["tui"]

    public enum ResolutionError: Error, Equatable, CustomStringConvertible {
        /// No file at the expected sibling path (typically: the app bundle was not built by
        /// `scripts/build_workbench_app.sh`, or the host was started from `.build/`).
        case missing(expectedPath: String)
        /// Present but not executable.
        case notExecutable(path: String)

        public var description: String {
            switch self {
            case .missing(let path):
                return "Bundled luma executable not found at \(path). "
                    + "Rebuild with rust/scripts/build_workbench_app.sh."
            case .notExecutable(let path):
                return "Bundled luma executable at \(path) is not executable."
            }
        }
    }

    /// The `luma` binary sits next to the host executable inside `Contents/MacOS/`.
    public static func expectedURL(hostExecutableURL: URL) -> URL {
        hostExecutableURL
            .resolvingSymlinksInPath()
            .deletingLastPathComponent()
            .appendingPathComponent(name)
    }

    /// - Parameters:
    ///   - hostExecutableURL: `Bundle.main.executableURL`, i.e.
    ///     `…/Luma.app/Contents/MacOS/LumaWorkbench`.
    ///   - fileExists: injected for tests; production passes `FileManager.default.fileExists`.
    ///   - isExecutable: injected for tests; production passes `FileManager.default.isExecutableFile`.
    public static func resolve(
        hostExecutableURL: URL,
        fileExists: (String) -> Bool,
        isExecutable: (String) -> Bool
    ) -> Result<URL, ResolutionError> {
        let candidate = expectedURL(hostExecutableURL: hostExecutableURL)
        let path = candidate.path
        guard fileExists(path) else {
            return .failure(.missing(expectedPath: path))
        }
        guard isExecutable(path) else {
            return .failure(.notExecutable(path: path))
        }
        return .success(candidate)
    }
}
