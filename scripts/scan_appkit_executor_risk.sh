#!/usr/bin/env bash
# Scans for Swift 6 / AppKit executor boundary violations. Exit non-zero on risk.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0

echo "== 1. @MainActor AppKit subclasses =="
if rg -n --type swift '@MainActor\s+(final |open |internal |public |private |fileprivate )?(class|final class)\s+\w+\s*:\s*NS(View|Panel|Window|WindowController|ViewController|Control|TextField|TextView|Button|ScrollView|StackView|TableViewCell|OutlineView|ClipView|Box|VisualEffectView|TableRowView|ImageView|SecureTextField)' Sources; then
  echo "ERROR: @MainActor on AppKit subclass (see docs/swift6-appkit-boundaries.md)"
  fail=1
fi

echo "== 2. AppKit overrides missing nonisolated =="
if rg -n --type swift '^\s*override func (layout|draw|hitTest|keyDown|cancelOperation|performKeyEquivalent|mouseDown|rightMouseDown|mouseEntered|mouseExited|updateTrackingAreas|viewDidChangeEffectiveAppearance|viewDidMoveToWindow|viewWillMoveToWindow|drawSelection|drawBackground|intrinsicContentSize|updateConstraints)\b' Sources/LumaApp \
   | rg -v 'nonisolated' >/tmp/luma_override_risk.txt || true; then
  if [ -s /tmp/luma_override_risk.txt ]; then
    echo "ERROR: AppKit override without nonisolated:"
    cat /tmp/luma_override_risk.txt
    fail=1
  fi
fi

echo "== 3. MainActor.assumeIsolated =="
if rg -n --type swift 'MainActor\.assumeIsolated' Sources; then
  echo "ERROR: MainActor.assumeIsolated is forbidden for AppKit/Carbon callbacks"
  fail=1
fi

echo "== 4. NotificationCenter selector observers in LumaApp =="
if rg -n --type swift 'addObserver\(\s*self,\s*selector:' Sources/LumaApp; then
  echo "ERROR: use LumaNotificationCenter.observe instead of selector observers"
  fail=1
fi

echo "== 5. AppKit subclass files missing @preconcurrency import =="
for f in $(rg -l --type swift 'class\s+\w+\s*:\s*NS(View|Panel|Window|Control|TextField|TextView|Button|ScrollView|StackView|Box|TableRowView)' Sources/LumaApp); do
  if ! head -15 "$f" | rg -q '@preconcurrency import AppKit'; then
    echo "ERROR: $f has AppKit subclass but no @preconcurrency import AppKit"
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "OK: no AppKit executor risks detected"
fi
exit "$fail"
