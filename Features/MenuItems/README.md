# Menu Bar Search

Menu Bar Search indexes the frontmost app's macOS menu bar through Accessibility and exposes cached menu items through `mb <query>` or `menu <query>`. Return performs the matched item with `AXPress`.

Configuration lives at:

`~/Library/Application Support/Luma/menu-items.json`

v1 supports `disabledBundleIDs` and keeps `pinnedItems` as a schema stub. The service refreshes on frontmost app changes, walks the AX menu tree with a 200 ms budget and five-level depth limit, and keeps results cached for 30 seconds.

The query hot path only reads cached records and fuzzy-scores title paths. No AX tree walking, shelling out, or AppleScript happens in `handle()`. Accessibility permission is required; denied permission should surface through Luma's existing permission banner and action error behavior.
