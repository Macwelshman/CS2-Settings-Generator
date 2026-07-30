#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="CS2 Settings Generator"
PROCESS_NAME="cs2-settings-generator"
BUNDLE_ID="com.macwelshman.cs2-settings-generator"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/apps/desktop/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_BINARY="$APP_MACOS/$PROCESS_NAME"
BUILD_BINARY="$ROOT_DIR/target/debug/$PROCESS_NAME"

pkill -x "$PROCESS_NAME" >/dev/null 2>&1 || true

cd "$ROOT_DIR"
cargo build -p cs2-settings-desktop --bin "$PROCESS_NAME"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES"
cp "$BUILD_BINARY" "$APP_BINARY"
cp "$ROOT_DIR/script/Info.plist" "$APP_CONTENTS/Info.plist"
cp "$ROOT_DIR/apps/desktop/src-tauri/icons/icon.png" "$APP_RESOURCES/AppIcon.png"
chmod +x "$APP_BINARY"

open_app() {
  /usr/bin/open -n "$APP_BUNDLE"
}

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$PROCESS_NAME\""
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    open_app
    for _ in 1 2 3 4 5; do
      if pgrep -x "$PROCESS_NAME" >/dev/null; then
        exit 0
      fi
      sleep 1
    done
    echo "$APP_NAME did not remain running after launch." >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac

