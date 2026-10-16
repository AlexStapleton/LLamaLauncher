#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -gt 1 || ( $# -eq 1 && "$1" != "--skip-checks" ) ]]; then printf 'Usage: %s [--skip-checks]\n' "$0" >&2; exit 1; fi
mkdir -p artifacts
if [[ "${1:-}" != "--skip-checks" ]]; then
npm test
npm run build
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --release --locked
fi
node tests/generate-notices.mjs
node tests/generate-settings-guide.mjs --check
node tests/verify-glib-backport.mjs
node tests/linux-package-inventory.mjs
cat > artifacts/portable-config.json <<'JSON'
{"bundle":{"active":true,"targets":["appimage"],"icon":["icons/icon.png"],"resources":["../LICENSE","../PORTABLE_RELEASES.md","../LLAMA_SETTINGS_GUIDE.md","artifacts/THIRD_PARTY_NOTICES.txt","artifacts/LINUX_PACKAGE_VERSIONS.txt"]}}
JSON
export APPIMAGE_EXTRACT_AND_RUN=1
npx --no-install tauri build --config artifacts/portable-config.json --bundles appimage -- --locked
version=$(node -p 'JSON.parse(require("fs").readFileSync("package.json","utf8")).version')
mapfile -t images < <(find "${CARGO_TARGET_DIR:-target}/release/bundle/appimage" -maxdepth 1 -name '*.AppImage' -type f)
[[ ${#images[@]} -eq 1 ]] || { echo 'Expected one AppImage output' >&2; exit 1; }
mkdir -p artifacts/release
image="artifacts/release/LLamaLauncher-$version-Linux-x86_64.AppImage"
cp "${images[0]}" "$image"
chmod +x "$image"
(cd artifacts/release && sha256sum "$(basename "$image")" > "$(basename "$image").sha256")
smoke=$(mktemp -d)
cp "$image" "$smoke/LLamaLauncher.AppImage"
xvfb-run -a bash tests/linux-window-smoke.sh "$smoke/LLamaLauncher.AppImage"
printf 'AppImage and checksum: %s\n' "$image"
