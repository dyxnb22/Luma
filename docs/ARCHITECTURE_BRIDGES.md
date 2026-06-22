# LauncherBridge vs ModuleDetailRegistry

## ModuleDetailRegistry (`LumaApp` only)

**Purpose:** Detail views read shared module instances and build `ModuleDetailView` implementations.

- Module references: `clipboardModule`, `notesModule`, `snippetsModule`, etc.
- `config`, `translation` for detail view construction
- `make(for: ModuleIdentifier)` factory
- `isLauncherQueryEmpty` launcher ↔ detail coordination flag

Detail views and `LauncherRootView` use this registry. **Modules must not import or call it.**

## LauncherBridge (`LumaModules`, injected by `LumaApp`)

**Purpose:** Actor-isolated modules invoke main-actor UI callbacks without importing AppKit.

- `openWordbookReview`, `openMediaDetail`, `reloadMediaDetail`, …
- `onSecretsLockStateChanged`
- `onBackFromDetail`, `onOpenSettings`, `onTranslateContentChanged` (detail navigation)

`AppCoordinator.start()` wires all closures once. New modules: add callbacks here, not to `ModuleDetailRegistry`.
