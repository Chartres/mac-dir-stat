#!/usr/bin/env bash
# Builds MacDirStat.app as a universal (x86_64 + arm64) bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-$(awk -F\" '/^version[[:space:]]*=/{print $2; exit}' "$ROOT/Cargo.toml")}"
APP_NAME="MacDirStat"
BIN_NAME="mac-dir-stat"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"

rustup target add x86_64-apple-darwin aarch64-apple-darwin >/dev/null

cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

lipo -create -output "$APP/Contents/MacOS/$BIN_NAME" \
    "$ROOT/target/x86_64-apple-darwin/release/$BIN_NAME" \
    "$ROOT/target/aarch64-apple-darwin/release/$BIN_NAME"

sed "s/@VERSION@/$VERSION/g" "$ROOT/macos/Info.plist.in" > "$APP/Contents/Info.plist"

# Code-sign with a Developer ID + hardened runtime when an identity is provided
# (CODESIGN_IDENTITY env). Required for notarization, which homebrew/cask needs.
# Without it the build is unsigned exactly as before — local dev stays friction-free.
if [ -n "${CODESIGN_IDENTITY:-}" ]; then
    echo "Signing with identity: $CODESIGN_IDENTITY"
    codesign --force --options runtime --timestamp \
        --sign "$CODESIGN_IDENTITY" "$APP/Contents/MacOS/$BIN_NAME"
    codesign --force --options runtime --timestamp \
        --sign "$CODESIGN_IDENTITY" "$APP"
    codesign --verify --strict --verbose=2 "$APP"
else
    echo "CODESIGN_IDENTITY not set — building unsigned (see docs/GROWTH.md Track A)."
fi

echo "Built $APP (version $VERSION)"
