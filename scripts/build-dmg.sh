#!/bin/sh
# Builds the app and its disk image without touching Finder.
#
# `tauri build` makes the DMG with create-dmg, whose AppleScript opens the
# mounted volume in Finder to arrange it — as a tab of the front window when
# Finder prefers tabs, resizing that window. Here Tauri builds only the .app
# and dmgbuild writes the same layout into the image directly
# (scripts/dmg-settings.py). dmgbuild lives in a venv under target/.
#
# Usage: scripts/build-dmg.sh
# Output: src-tauri/target/release/bundle/dmg/<product>_<version>_<arch>.dmg
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

DMGBUILD_VERSION=1.6.7

product=$(node -p "require('./src-tauri/tauri.conf.json').productName")
version=$(node -p "require('./package.json').version")
case $(uname -m) in
    arm64) arch=aarch64 ;;
    *) arch=$(uname -m) ;;
esac
bundle=src-tauri/target/release/bundle
venv=src-tauri/target/dmgbuild-venv

npx tauri build --bundles app

if ! "$venv/bin/dmgbuild" --help >/dev/null 2>&1; then
    python3 -m venv "$venv"
    "$venv/bin/pip" install --quiet "dmgbuild==$DMGBUILD_VERSION"
fi

mkdir -p "$bundle/dmg"
out="$bundle/dmg/${product}_${version}_${arch}.dmg"
rm -f "$out"
"$venv/bin/dmgbuild" \
    -s scripts/dmg-settings.py \
    -D app="$bundle/macos/$product.app" \
    -D icon=src-tauri/icons/icon.icns \
    "$product" "$out"

echo "Built: $out"
