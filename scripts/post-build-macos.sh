#!/bin/bash
# post-build-macos.sh — Signs the app with ad-hoc and patches Info.plist.
# Run after `bun run tauri build`.
#
# Usage:
#   ./scripts/post-build-macos.sh [path-to-app-bundle] [--production]
#
# By default this uses `entitlements-dev.plist` (no sandbox) so that
# Finder "Open With" / file associations work for local testing.
# Pass --production to use the sandboxed `entitlements.plist`
# (only meaningful when shipping under a paid Developer ID).

set -euo pipefail

APP_BUNDLE="${1:-}"
USE_PRODUCTION_ENTITLEMENTS=false

# Parse args
for arg in "$@"; do
  case "$arg" in
    --production)
      USE_PRODUCTION_ENTITLEMENTS=true
      shift
      ;;
  esac
done

if [ -z "$APP_BUNDLE" ]; then
  RELEASE_BUNDLE="src-tauri/target/release/bundle/macos/text-rs.app"
  DEBUG_BUNDLE="src-tauri/target/debug/bundle/macos/text-rs.app"

  if [ -d "$RELEASE_BUNDLE" ]; then
    APP_BUNDLE="$RELEASE_BUNDLE"
  elif [ -d "$DEBUG_BUNDLE" ]; then
    APP_BUNDLE="$DEBUG_BUNDLE"
  else
    echo "Error: Could not find .app bundle. Provide the path as an argument."
    echo "Usage: $0 /path/to/text-rs.app [--production]"
    exit 1
  fi
fi

INFO_PLIST="$APP_BUNDLE/Contents/Info.plist"

if [ ! -f "$INFO_PLIST" ]; then
  echo "Error: Info.plist not found at $INFO_PLIST"
  exit 1
fi

echo "=== Step 1: Patching Info.plist ==="
/usr/libexec/PlistBuddy -c "Add :NSDesktopFolderUsageDescription string 'text-rs needs access to your Desktop to open and save files.'" "$INFO_PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :NSDocumentsFolderUsageDescription string 'text-rs needs access to your Documents folder to open and save files.'" "$INFO_PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :NSDownloadsFolderUsageDescription string 'text-rs needs access to your Downloads folder to open and save files.'" "$INFO_PLIST" 2>/dev/null || true
echo "Info.plist updated."

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ "$USE_PRODUCTION_ENTITLEMENTS" = "true" ]; then
  ENTITLEMENTS="$PROJECT_ROOT/src-tauri/entitlements.plist"
  echo "Using PRODUCTION entitlements (sandboxed) — file associations require Developer ID signing."
else
  ENTITLEMENTS="$PROJECT_ROOT/src-tauri/entitlements-dev.plist"
  echo "Using DEV entitlements (no sandbox) — file associations work with ad-hoc signing."
fi

echo "=== Step 2: Ad-hoc code signing with entitlements ==="
if [ -f "$ENTITLEMENTS" ]; then
  codesign --force --deep --sign - --entitlements "$ENTITLEMENTS" "$APP_BUNDLE"
  echo "Signed with entitlements: $APP_BUNDLE"
else
  echo "Warning: $ENTITLEMENTS not found"
  codesign --force --deep --sign - "$APP_BUNDLE"
  echo "Signed without entitlements: $APP_BUNDLE"
fi

echo "=== Step 3: Verifying signature ==="
codesign --verify --verbose=2 "$APP_BUNDLE" 2>&1 || echo "Warning: verification returned warnings (expected for ad-hoc)"

echo ""
echo "Done! App is signed and ready to run."
echo "  open $APP_BUNDLE"
