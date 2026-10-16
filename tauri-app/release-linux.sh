#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -gt 1 || ( $# -eq 1 && "$1" != '--skip-clean-host' ) ]]; then echo 'Usage: release-linux.sh [--skip-clean-host]' >&2; exit 1; fi
# The shared workflow has already run the complete source checks.
bash package-appimage.sh --skip-checks
# Confirm the runtime that was missing in the old ZIP is actually inside.
image=$(realpath "artifacts/release/LLamaLauncher-$(node -p 'JSON.parse(require("fs").readFileSync("package.json","utf8")).version')-Linux-x86_64.AppImage")
fixture=$(mktemp -d)
(cd "$fixture" && "$image" --appimage-extract > extraction.log)
[[ -n "$(find "$fixture/squashfs-root" -name 'libwebkit2gtk-4.1.so.0*' -print -quit)" ]] || { echo 'AppImage is missing WebKitGTK' >&2; exit 1; }
[[ -n "$(find "$fixture/squashfs-root" -name 'WebKitWebProcess' -print -quit)" ]] || { echo 'AppImage is missing the WebKit process helper' >&2; exit 1; }
[[ -n "$(find "$fixture/squashfs-root" -name 'libsoup-3.0.so*' -print -quit)" ]] || { echo 'AppImage is missing libsoup 3' >&2; exit 1; }
[[ -z "$(find "$fixture/squashfs-root" \( -name 'libsoup-2.4.so*' -o -name 'libwebkit2gtk-4.0.so*' \) -print -quit)" ]] || { echo 'AppImage unexpectedly contains the deprecated WebKit/libsoup stack' >&2; exit 1; }
if [[ "${1:-}" != '--skip-clean-host' ]]; then bash tests/appimage-clean-host.sh "$image"; fi
