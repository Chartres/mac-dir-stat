#!/usr/bin/env bash
# Renders assets/icon/icon.svg to assets/icon/AppIcon.icns and the 256px window icon.
# macOS only: qlmanage draws the SVG, sips resizes, iconutil builds the .icns.
# The outputs are committed; run this only after editing icon.svg.
set -euo pipefail

DIR="$(cd "$(dirname "$0")/../assets/icon" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

qlmanage -t -s 1024 -o "$WORK" "$DIR/icon.svg" >/dev/null 2>&1
BASE="$WORK/icon.svg.png"
[ -f "$BASE" ] || { echo "qlmanage did not render icon.svg" >&2; exit 1; }

SET="$WORK/AppIcon.iconset"
mkdir "$SET"
for s in 16 32 128 256 512; do
    sips -z $s $s "$BASE" --out "$SET/icon_${s}x${s}.png" >/dev/null
    sips -z $((s * 2)) $((s * 2)) "$BASE" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$DIR/AppIcon.icns"
sips -z 256 256 "$BASE" --out "$DIR/icon-256.png" >/dev/null
echo "Wrote $DIR/AppIcon.icns and $DIR/icon-256.png"
