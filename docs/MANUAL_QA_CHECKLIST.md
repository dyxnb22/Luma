# Manual QA Checklist

## Hotkey

- Works after launch.
- Default chord is Command+Space.
- Toggles panel when panel is visible.
- Still works after sleep/wake.
- Does not collide with Spotlight or input method shortcuts.

## Panel

- Appears on active display.
- Appears over fullscreen apps.
- Appears on all Spaces as expected.
- Query field is focused immediately.
- Escape dismisses.
- Focus loss behavior matches Settings.

## Performance

- Hotkey -> panel p95 <= 50 ms.
- Keystroke -> first result p95 <= 30 ms.
- No visible row jumping while typing.

## Permissions

- Accessibility denied state is clear.
- Accessibility granted state enables window focus.
- Settings links to System Settings when permission is missing.

## Clipboard

- Does not store concealed/transient pasteboard values.
- Skips the entire pasteboard change if any blocked type is present.
- Enforces 500-entry, 7-day, and 100-KB body caps.
- Enforces retention caps.
- Clear history command works.
