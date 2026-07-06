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
MAINACTOR_OBJC_AWK='function count_braces(line,    t, opens, closes) {
  t = line
  opens = gsub(/{/, "{", t)
  t = line
  closes = gsub(/}/, "}", t)
  return opens - closes
}
function is_class_line(line) {
  return line ~ /^[[:space:]]*(private[[:space:]]+|public[[:space:]]+|internal[[:space:]]+|fileprivate[[:space:]]+)?(final[[:space:]]+)?(class|actor)[[:space:]]+/
}
function is_single_line_mainactor_class(line) {
  return line ~ /^[[:space:]]*@MainActor[[:space:]]+(private[[:space:]]+|public[[:space:]]+|internal[[:space:]]+|fileprivate[[:space:]]+)?(final[[:space:]]+)?(class|actor)[[:space:]]+/
}
function is_pending_mainactor_line(line) {
  return line ~ /^[[:space:]]*@MainActor[[:space:]]*$/
}
function is_objc_func(line) {
  return line ~ /@objc[[:space:]]+(private[[:space:]]+)?func[[:space:]]+/
}
function is_bridged_objc(line, prev) {
  if (line ~ /nonisolated/) return 1
  if (line ~ /@objc[[:space:]]+nonisolated/) return 1
  if (prev ~ /nonisolated[[:space:]]*$/) return 1
  return 0
}
{
  line = $0
  if (is_single_line_mainactor_class(line) || (pending_mainactor && is_class_line(line))) {
    in_mainactor = 1
    mainactor_close_depth = depth
    pending_mainactor = 0
  } else if (is_pending_mainactor_line(line)) {
    pending_mainactor = 1
  } else if (pending_mainactor) {
    pending_mainactor = 0
  }
  if (in_mainactor && is_objc_func(line) && !is_bridged_objc(line, prev_line)) {
    printf "WARN: %s:%d: @objc target/action inside @MainActor type should be nonisolated: %s\n", FILENAME, NR, line
  }
  depth += count_braces(line)
  if (in_mainactor && depth <= mainactor_close_depth) {
    in_mainactor = 0
  }
  prev_line = line
}'
for f in $(rg -l --type swift '.' Sources/LumaApp/Launcher Sources/LumaApp/Settings 2>/dev/null || true); do
  awk -v FILENAME="$f" "$MAINACTOR_OBJC_AWK" "$f"
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
