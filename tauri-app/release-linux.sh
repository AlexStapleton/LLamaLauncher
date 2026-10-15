#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
node -e 'if(JSON.parse(require("fs").readFileSync("package.json","utf8")).version!=="1.1.0")throw Error("Expected version 1.1.0")'
node tests/generate-settings-guide.mjs --check
node tests/generate-notices.mjs
stage=artifacts/release-stage/LLamaLauncher
mkdir -p "$stage" artifacts/release
cp target/release/llama-server-configurator "$stage/"
chmod +x "$stage/llama-server-configurator"
cp ../LICENSE ../PORTABLE_RELEASES.md ../LLAMA_SETTINGS_GUIDE.md artifacts/THIRD_PARTY_NOTICES.txt "$stage/"
cat > "$stage/START_HERE.txt" <<'TXT'
Run ./llama-server-configurator on Linux x86_64 with GTK 3, WebKitGTK 4.0
and AppIndicator libraries installed (Ubuntu 22.04 build target).
This native ZIP is not an AppImage and does not bundle system GUI libraries.
Choose/download llama.cpp in Launch and select a GGUF in Models.
Keep data beside the executable when upgrading; model weights are not bundled.
Read LLAMA_SETTINGS_GUIDE.md offline for setting explanations and examples.
TXT
(cd artifacts/release-stage && zip -qr ../release/LLamaLauncher-1.1-Linux-x86_64.zip LLamaLauncher)
(cd artifacts/release && sha256sum LLamaLauncher-1.1-Linux-x86_64.zip > LLamaLauncher-1.1-Linux-x86_64.zip.sha256)
