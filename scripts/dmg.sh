#!/usr/bin/env bash
# Wraps MacDirStat.app into a DMG with a drag-to-Applications layout.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-$(awk -F\" '/^version[[:space:]]*=/{print $2; exit}' "$ROOT/Cargo.toml")}"
APP_NAME="MacDirStat"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"
DMG="$DIST/$APP_NAME-$VERSION.dmg"
STAGE="$DIST/dmg-stage"

[ -d "$APP" ] || { echo "Missing $APP — run scripts/bundle.sh first" >&2; exit 1; }

rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

hdiutil create -volname "$APP_NAME $VERSION" \
    -srcfolder "$STAGE" \
    -ov -format ULFO \
    "$DMG" >/dev/null

rm -rf "$STAGE"
echo "Created $DMG"
