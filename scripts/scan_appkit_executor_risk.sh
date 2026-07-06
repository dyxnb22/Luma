#!/usr/bin/env bash
# Scans for Swift 6 / AppKit executor boundary violations. Exit non-zero on risk.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
APPKIT_BASE='NS(View|Panel|Window|WindowController|ViewController|Control|TextField|TextView|Button|ScrollView|StackView|TableViewCell|OutlineView|ClipView|Box|VisualEffectView|TableRowView|ImageView|SecureTextField|StackView|TableCellView)'
# Project AppKit wrappers inherit NS* types; treat them like direct AppKit subclasses.
LUMA_APPKIT_BASE="${APPKIT_BASE}|LumaWindow|LauncherPanel"

echo "== 1. @MainActor AppKit subclasses (single-line and multiline) =="
if rg -U --multiline-dotall -n --type swift \
  "@MainActor\\s*\\n\\s*(?:final\\s+|open\\s+|internal\\s+|public\\s+|private\\s+|fileprivate\\s+)?(?:class|final class)\\s+\\w+\\s*:\\s*(${LUMA_APPKIT_BASE})" \
  Sources/LumaApp; then
  echo "ERROR: @MainActor on AppKit subclass (see docs/swift6-appkit-boundaries.md)"
  fail=1
fi
if rg -n --type swift "@MainActor\\s+(?:final\\s+|open\\s+|internal\\s+|public\\s+|private\\s+|fileprivate\\s+)?(?:class|final class)\\s+\\w+\\s*:\\s*(${LUMA_APPKIT_BASE})" Sources/LumaApp; then
  echo "ERROR: @MainActor on AppKit subclass (single-line)"
  fail=1
fi

echo "== 2. @MainActor NSViewController subclasses =="
if rg -U --multiline-dotall -n --type swift \
  "@MainActor\\s*\\n\\s*(?:final\\s+|open\\s+|internal\\s+|public\\s+|private\\s+|fileprivate\\s+)?(?:class|final class)\\s+\\w+\\s*:\\s*NSViewController" \
  Sources/LumaApp; then
  echo "ERROR: @MainActor on NSViewController — use nonisolated loadView + Task { @MainActor }"
  fail=1
fi
if rg -n --type swift "@MainActor\\s+(?:final\\s+)?(?:class|final class)\\s+\\w+\\s*:\\s*NSViewController" Sources/LumaApp; then
  echo "ERROR: @MainActor on NSViewController (single-line)"
  fail=1
fi

echo "== 3. AppKit func overrides missing nonisolated =="
FUNC_OVERRIDES='layout|draw|hitTest|keyDown|cancelOperation|performKeyEquivalent|mouseDown|rightMouseDown|mouseEntered|mouseExited|updateTrackingAreas|viewDidChangeEffectiveAppearance|viewDidMoveToWindow|viewWillMoveToWindow|drawSelection|drawBackground|updateConstraints|loadView|viewDidLoad|viewWillAppear|awakeFromNib|becomeKey|resignKey|becomeMain|resignMain|setFrame|setContentSize|insertText'
if rg -n --type swift "^\s*override func (${FUNC_OVERRIDES})\b" Sources/LumaApp \
   | rg -v 'nonisolated' >/tmp/luma_override_risk.txt || true; then
  if [ -s /tmp/luma_override_risk.txt ]; then
    echo "ERROR: AppKit override without nonisolated:"
    cat /tmp/luma_override_risk.txt
    fail=1
  fi
fi

echo "== 3b. AppKit property overrides missing nonisolated =="
PROPERTY_OVERRIDES='isFlipped|acceptsFirstResponder|isOpaque|intrinsicContentSize|canBecomeKey|canBecomeMain|string|stringValue'
if rg -n --type swift "^\s*override (class )?var (${PROPERTY_OVERRIDES})\b" Sources/LumaApp \
   | rg -v 'nonisolated' >/tmp/luma_property_override_risk.txt || true; then
  if [ -s /tmp/luma_property_override_risk.txt ]; then
    echo "ERROR: AppKit property override without nonisolated:"
    cat /tmp/luma_property_override_risk.txt
    fail=1
  fi
fi

echo "== 3c. @MainActor annotation on AppKit overrides =="
if rg -U -n --type swift "@MainActor\s*\n\s*(nonisolated\s+)?override\s+(func|var)" Sources/LumaApp; then
  echo "ERROR: @MainActor on AppKit override; use nonisolated entry + Task { @MainActor }"
  fail=1
fi

echo "== 7. @MainActor NSObject target/action entrypoints (warn-only) =="
for f in $(rg -l --type swift '@MainActor' Sources/LumaApp/Launcher Sources/LumaApp/Settings 2>/dev/null || true); do
  if ! rg -q '@MainActor\s+(?:final\s+)?class' "$f"; then
    continue
  fi
  while IFS= read -r line; do
    lineno="${line%%:*}"
    rest="${line#*:}"
    if echo "$rest" | rg -q 'nonisolated'; then
      continue
    fi
    if echo "$rest" | rg -q '@objc nonisolated'; then
      continue
    fi
    prevline=""
    if [ "$lineno" -gt 1 ]; then
      prevline=$(sed -n "$((lineno - 1))p" "$f")
    fi
    if echo "$prevline" | rg -q 'nonisolated'; then
      continue
    fi
    echo "WARN: $f:$lineno: @objc target/action inside @MainActor type should be nonisolated: $rest"
  done < <(rg -n --type swift '@objc\s+(private\s+)?func' "$f" || true)
done

echo "== 4. MainActor.assumeIsolated =="
if rg -n --type swift 'MainActor\.assumeIsolated' Sources; then
  echo "ERROR: MainActor.assumeIsolated is forbidden for AppKit/Carbon callbacks"
  fail=1
fi

echo "== 5. NotificationCenter selector observers (single-line and multiline) =="
if rg -U --multiline-dotall -n --type swift 'addObserver\(\s*self,\s*selector:' Sources/LumaApp; then
  echo "ERROR: use LumaNotificationCenter.observe instead of selector observers"
  fail=1
fi

echo "== 6. AppKit subclass files missing @preconcurrency import =="
for f in $(rg -l --type swift "class\\s+\\w+\\s*:\\s*(${LUMA_APPKIT_BASE})" Sources/LumaApp); do
  if ! head -15 "$f" | rg -q '@preconcurrency import AppKit'; then
    echo "ERROR: $f has AppKit subclass but no @preconcurrency import AppKit"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "OK: no AppKit executor risks detected"
fi
exit "$fail"
