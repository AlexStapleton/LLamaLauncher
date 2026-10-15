#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -ne 0 ]]; then printf 'Usage: %s\n' "$0" >&2; exit 1; fi
mkdir -p artifacts
npm test
node tests/verify-glib-backport.mjs
npm run build
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --release --locked
node tests/generate-notices.mjs
node tests/generate-settings-guide.mjs --check
cat > artifacts/portable-config.json <<'JSON'
{"tauri":{"bundle":{"active":true,"identifier":"io.github.alexstapleton.llamalauncher","targets":["appimage"],"icon":["icons/icon.png"],"resources":["../LICENSE","../PORTABLE_RELEASES.md","../LLAMA_SETTINGS_GUIDE.md","artifacts/THIRD_PARTY_NOTICES.txt"]}}}
JSON
cargo tauri build --config artifacts/portable-config.json --bundles appimage -- --locked
find "${CARGO_TARGET_DIR:-target}/release/bundle/appimage" -name '*.AppImage' -type f -exec cp '{}' artifacts/ \;
for image in artifacts/*.AppImage; do sha256sum "$image" > "$image.sha256"; xvfb-run -a bash tests/linux-window-smoke.sh "$image"; done
printf 'AppImage and checksums are in artifacts/. Test with FUSE and --appimage-extract-and-run.\n'
