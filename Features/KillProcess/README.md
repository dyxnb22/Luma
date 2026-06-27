# Kill Process

Kill Process lists running GUI apps and lets the user quit, force kill, or relaunch them from the launcher. Triggers are `kill`, `quit`, and `k`; `kill` is the discoverable default.

The module uses `NSWorkspace.shared.runningApplications`, filters Luma itself, and displays bundle ID plus resident memory. Return sends a normal `terminate()`. Secondary actions include Force Kill with second-modifier confirmation and Relaunch, which waits briefly for the previous PID to disappear before opening the bundle again.

There is no config file in v1. The module does not enumerate daemons with `ps`, does not call `kill(2)`, and does not show non-GUI processes. Finder, Dock, WindowServer, and SystemUIServer are guarded with additional confirmation.
