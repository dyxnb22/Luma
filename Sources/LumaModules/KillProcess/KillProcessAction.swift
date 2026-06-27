import Darwin
import Foundation

public enum KillProcessAction: Codable, Sendable, Hashable {
    case quit(pid: pid_t)
    case forceKill(pid: pid_t)
    case relaunch(bundleID: String, pid: pid_t)
}
