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

echo "Built $APP (version $VERSION)"
