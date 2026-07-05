#!/usr/bin/env bash
# Shared helpers for assembling and signing Luma.app.

luma_resolve_sign_identity() {
  local identity="${LUMA_CODESIGN_IDENTITY:-}"
  if [[ -z "$identity" ]]; then
    identity="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '\"' '/Developer ID Application/ { print $2; exit }')"
  fi
  if [[ -z "$identity" ]]; then
    identity="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '\"' '/Apple Development/ { print $2; exit }')"
  fi
  if [[ -z "$identity" ]]; then
    identity="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '\"' '/Luma Local Development/ { print $2; exit }')"
  fi
  if [[ -z "$identity" ]]; then
    identity="-"
  fi
  printf '%s' "$identity"
}

luma_swift_build_release() {
  local root="$1"
  cd "$root"
  local flags=()
  if [[ -n "${LUMA_SWIFT_BUILD_FLAGS:-}" ]]; then
    read -r -a flags <<< "$LUMA_SWIFT_BUILD_FLAGS"
  fi
  if [[ "${LUMA_WARNINGS_AS_ERRORS:-}" == "1" ]]; then
    flags+=(-Xswiftc -warnings-as-errors)
  fi
  if ((${#flags[@]} > 0)); then
    swift build -c release "${flags[@]}"
  else
    swift build -c release
  fi
}

luma_assemble_app() {
  local root="$1"
  local app_dir="$2"
  local macos_dir="$app_dir/Contents/MacOS"
  local resources_dir="$app_dir/Contents/Resources"
  local entitlements="$root/Resources/Luma.entitlements"
  local info_plist="$root/Resources/Info.plist"

  rm -rf "$app_dir"
  mkdir -p "$macos_dir" "$resources_dir"
  cp "$root/.build/release/Luma" "$macos_dir/Luma"
  cp "$info_plist" "$app_dir/Contents/Info.plist"

  local identity
  identity="$(luma_resolve_sign_identity)"
  if [[ "$identity" == "-" ]]; then
    echo "No stable code-signing identity found; using ad-hoc signing. Run ./scripts/install_local_codesign_cert.sh to stabilize Accessibility across rebuilds."
  else
    echo "Signing with: $identity"
  fi

  local sign_args=(--force --options runtime --sign "$identity")
  if [[ -f "$entitlements" ]]; then
    sign_args+=(--entitlements "$entitlements")
  fi

  codesign "${sign_args[@]}" "$macos_dir/Luma"
  codesign "${sign_args[@]}" "$app_dir"
  codesign --verify --verbose=2 "$app_dir"
}
