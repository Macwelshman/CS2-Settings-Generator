#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILDS_DIR="$ROOT_DIR/Builds"
BUNDLE_DIR="$ROOT_DIR/target/release/bundle/dmg"
MODE="${1:-build}"

mkdir -p "$BUILDS_DIR"
cd "$ROOT_DIR"

case "$MODE" in
  build)
    bash "$ROOT_DIR/script/compile_macos_icon.sh"
    BUILD_MARKER="$(mktemp "${TMPDIR:-/tmp}/cs2-settings-build.XXXXXX")"
    trap 'rm -f "$BUILD_MARKER"' EXIT
    cargo tauri build --bundles dmg
    DMG_PATH="$(find "$BUNDLE_DIR" -maxdepth 1 -type f -name '*.dmg' -newer "$BUILD_MARKER" -print -quit)"
    ;;
  --collect-only|collect-only)
    DMG_PATH="$(find "$BUNDLE_DIR" -maxdepth 1 -type f -name '*.dmg' -print -quit)"
    ;;
  *)
    echo "usage: $0 [--collect-only]" >&2
    exit 2
    ;;
esac

if [[ -z "$DMG_PATH" ]]; then
  echo "The release build completed without producing a DMG." >&2
  exit 1
fi

cp "$DMG_PATH" "$BUILDS_DIR/"
echo "Release build ready: $BUILDS_DIR/$(basename "$DMG_PATH")"

# A whole, signed app bundle is required for the in-app replacement utility.
UPDATE_APP="$ROOT_DIR/target/release/bundle/macos/CS2 Settings Generator.app"
UPDATE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$UPDATE_APP/Contents/Info.plist")"
UPDATE_ARCH="$(uname -m)"
case "$UPDATE_ARCH" in arm64) ;; x86_64) UPDATE_ARCH=x64 ;; *) exit 1 ;; esac
/usr/bin/codesign --verify --deep --strict "$UPDATE_APP"
/usr/bin/lipo "$UPDATE_APP/Contents/MacOS/cs2-settings-generator" -verify_arch "$(uname -m)"
UPDATE_ZIP="$BUILDS_DIR/CS2-Settings-Generator-$UPDATE_VERSION-macos-$UPDATE_ARCH.zip"
# ditto preserves the signature and executable permissions. Avoid AppleDouble sidecars.
/usr/bin/ditto -c -k --norsrc --keepParent "$UPDATE_APP" "$UPDATE_ZIP"
/usr/bin/shasum -a 256 "$UPDATE_ZIP" > "$UPDATE_ZIP.sha256"
echo "Update package ready: $UPDATE_ZIP"
