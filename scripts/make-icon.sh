#!/usr/bin/env bash
# Renders assets/icon/icon.svg to assets/icon/AppIcon.icns and the 256px window icon.
# macOS only: render-svg.swift draws the SVG on a transparent canvas (qlmanage fills
# white, which squared off the corners), sips resizes, iconutil builds the .icns.
# The outputs are committed; run this only after editing icon.svg.
set -euo pipefail

DIR="$(cd "$(dirname "$0")/../assets/icon" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

BASE="$WORK/icon.png"
swift "$(dirname "$0")/render-svg.swift" "$DIR/icon.svg" "$BASE" 1024

SET="$WORK/AppIcon.iconset"
mkdir "$SET"
for s in 16 32 128 256 512; do
    sips -z $s $s "$BASE" --out "$SET/icon_${s}x${s}.png" >/dev/null
    sips -z $((s * 2)) $((s * 2)) "$BASE" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$DIR/AppIcon.icns"
sips -z 256 256 "$BASE" --out "$DIR/icon-256.png" >/dev/null
echo "Wrote $DIR/AppIcon.icns and $DIR/icon-256.png"
