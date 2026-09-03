#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICON_OUTPUT="${1:-$ROOT_DIR/apps/desktop/src-tauri/icons/macos-compiled}"
mkdir -p "$ICON_OUTPUT"
xcrun actool --compile "$ICON_OUTPUT" --platform macosx \
  --minimum-deployment-target 14.0 --app-icon CS2Settings \
  --output-partial-info-plist "$ICON_OUTPUT/partial.plist" \
  "$ROOT_DIR/apps/desktop/src-tauri/icons/CS2Settings.icon"
cp "$ICON_OUTPUT/CS2Settings.icns" "$ROOT_DIR/apps/desktop/src-tauri/icons/icon.icns"
