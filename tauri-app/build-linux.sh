#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
npm test
npm run build
cargo check --locked
cargo build --release --locked
mkdir -p desktop-app
cp "${CARGO_TARGET_DIR:-target}/release/llama-server-configurator" desktop-app/llama-server-configurator
printf 'Linux executable: %s/desktop-app/llama-server-configurator\n' "$PWD"
