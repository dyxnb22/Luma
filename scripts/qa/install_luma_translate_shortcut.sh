#!/usr/bin/env bash
# Install a signed "Luma Translate" shortcut (Translate Text action) for Shortcuts fallback.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
UNSIGNED="$ROOT/scripts/qa/LumaTranslate.shortcut"
SIGNED="$ROOT/scripts/qa/LumaTranslate.signed.shortcut"

if shortcuts list 2>/dev/null | grep -qx "Luma Translate"; then
  echo "Luma Translate shortcut already installed"
  exit 0
fi

cat > "$UNSIGNED" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>WFWorkflowActions</key>
  <array>
    <dict>
      <key>WFWorkflowActionIdentifier</key>
      <string>is.workflow.actions.detectlanguage</string>
      <key>WFWorkflowActionParameters</key>
      <dict/>
    </dict>
    <dict>
      <key>WFWorkflowActionIdentifier</key>
      <string>is.workflow.actions.translate</string>
      <key>WFWorkflowActionParameters</key>
      <dict>
        <key>WFTranslateTextActionSourceLanguage</key>
        <dict>
          <key>WFLanguageChoice</key>
          <string>Automatic</string>
        </dict>
        <key>WFTranslateTextActionDestinationLanguage</key>
        <dict>
          <key>WFLanguageChoice</key>
          <string>zh-Hans</string>
        </dict>
      </dict>
    </dict>
  </array>
  <key>WFWorkflowClientVersion</key>
  <string>900</string>
  <key>WFWorkflowMinimumClientVersion</key>
  <integer>900</integer>
  <key>WFWorkflowName</key>
  <string>Luma Translate</string>
  <key>WFWorkflowTypes</key>
  <array>
    <string>NCWidget</string>
    <string>WatchKit</string>
  </array>
</dict>
</plist>
PLIST

if shortcuts sign -i "$UNSIGNED" -o "$SIGNED" 2>/dev/null; then
  shortcuts view "$SIGNED" 2>/dev/null || true
  echo "Signed Luma Translate shortcut — confirm import in Shortcuts if prompted"
else
  echo "warn: could not sign shortcut; duplicate 翻译文本 manually and rename to Luma Translate" >&2
  exit 1
fi

if shortcuts list 2>/dev/null | grep -qx "Luma Translate"; then
  echo "Luma Translate ready"
  echo "hello" | shortcuts run "Luma Translate" 2>&1 | head -3 || true
fi
