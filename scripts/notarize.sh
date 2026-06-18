#!/usr/bin/env bash
# Notarizes and staples a DMG (or .app) with Apple's notary service.
#
# Reads credentials from the environment (set these as CI secrets):
#   MACOS_NOTARY_APPLE_ID   Apple ID email of the Developer account
#   MACOS_NOTARY_PASSWORD   app-specific password for that Apple ID
#   MACOS_NOTARY_TEAM_ID    10-char Developer Team ID
#
# No-op (with a warning) when credentials are absent, so unsigned local/CI
# builds keep working. homebrew/cask requires a notarized, Gatekeeper-passing
# app — see docs/GROWTH.md Track A.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-$(awk -F\" '/^version[[:space:]]*=/{print $2; exit}' "$ROOT/Cargo.toml")}"
DMG="${2:-$ROOT/dist/MacDirStat-$VERSION.dmg}"

if [ -z "${MACOS_NOTARY_APPLE_ID:-}" ] || [ -z "${MACOS_NOTARY_PASSWORD:-}" ] || [ -z "${MACOS_NOTARY_TEAM_ID:-}" ]; then
    echo "Notary credentials not set (MACOS_NOTARY_APPLE_ID / _PASSWORD / _TEAM_ID)."
    echo "Skipping notarization — the DMG ships unsigned. See docs/GROWTH.md Track A."
    exit 0
fi

[ -f "$DMG" ] || { echo "Missing DMG: $DMG — run scripts/dmg.sh first" >&2; exit 1; }

echo "Submitting $DMG to Apple notary service (this can take a few minutes)…"
xcrun notarytool submit "$DMG" \
    --apple-id "$MACOS_NOTARY_APPLE_ID" \
    --password "$MACOS_NOTARY_PASSWORD" \
    --team-id "$MACOS_NOTARY_TEAM_ID" \
    --wait

echo "Stapling the notarization ticket onto the DMG…"
xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"
echo "Notarized + stapled: $DMG"
