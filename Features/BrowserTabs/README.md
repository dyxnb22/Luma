# Browser Tabs

Browser Tabs searches cached open tabs from Safari, Chrome, Brave, Edge, and Arc. It is disabled by default because macOS Automation prompts are intrusive; enable it in Settings → Modules, then use `tab <query>` or `tabs <query>`.

The service uses per-browser AppleScript adapters with an 800 ms timeout and caches tab records for five seconds. Records include browser bundle ID, window index, tab index, title, and URL. Return activates the browser, raises the window, and selects the target tab.

v1 does not run AppleScript on every keystroke. Refresh happens during module warmup and through the service cache path. If Automation permission is denied, macOS owns the permission prompt and the module reports a failed action without exposing raw script output.
