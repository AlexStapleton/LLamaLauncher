#!/usr/bin/env bash
set -euo pipefail
# Fresh profile: no real credentials, MCP endpoint or model processes.
fixture="$(mktemp -d)"
export XDG_CONFIG_HOME="$fixture/config"
export XDG_CACHE_HOME="$fixture/cache"
export XDG_DATA_HOME="$fixture/data"
export LLAMA_LAUNCHER_DATA_DIR="$fixture/launcher-data"
export XDG_RUNTIME_DIR="$fixture/runtime"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export WEBKIT_DISABLE_DMABUF_RENDERER=1
if [[ $# -gt 0 ]]; then
  "$1" --appimage-extract-and-run >"$fixture/stdout" 2>"$fixture/stderr" &
else
  target/release/llama-server-configurator >"$fixture/stdout" 2>"$fixture/stderr" &
fi
launcher_pid=$!
trap 'kill "$launcher_pid" 2>/dev/null || true; wait "$launcher_pid" 2>/dev/null || true' EXIT
sleep 5
kill -0 "$launcher_pid"
if grep -E 'panicked at|Unable to initialize|error while loading shared libraries' "$fixture/stderr"; then exit 1; fi
echo 'Native Linux launcher stayed alive in the virtual display for five seconds.'
# Keep logs as artifacts; do not recursively delete computed paths.
mkdir -p artifacts
cp "$fixture/stderr" artifacts/linux-window-stderr.log
