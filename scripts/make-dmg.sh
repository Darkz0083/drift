#!/bin/sh
# Builds the distributable disk image from an already-built app.
#
# Tauri's own DMG target shells out to `create-dmg`, which finds the mounted
# volume by grepping `hdiutil info` for a device name. That lookup fails here —
# "unable to proceed ... interstitial disk image was not found" — and the script
# is regenerated on every build, so it cannot be patched. `hdiutil create` does
# the same job without the volume hunt, without Finder, and without AppleScript,
# which also means it works over SSH and in CI where there is no GUI session.
#
# Usage: scripts/make-dmg.sh <Drift.app> <output.dmg>
set -eu

APP=${1:?usage: make-dmg.sh <Drift.app> <output.dmg>}
OUT=${2:?usage: make-dmg.sh <Drift.app> <output.dmg>}

if [ ! -d "$APP" ]; then
  echo "no app bundle at $APP" >&2
  exit 1
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

# A staged copy, so the image holds the app beside a link to /Applications and
# nothing else from the build directory.
cp -R "$APP" "$WORK/"
ln -s /Applications "$WORK/Applications"

rm -f "$OUT"
hdiutil create \
  -volname "Drift" \
  -srcfolder "$WORK" \
  -fs HFS+ \
  -format UDZO \
  -ov \
  "$OUT" >/dev/null

echo "$OUT"
