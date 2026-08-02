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
