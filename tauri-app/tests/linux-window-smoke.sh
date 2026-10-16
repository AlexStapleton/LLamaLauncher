#!/usr/bin/env bash
set -euo pipefail
# Fresh profile: no real credentials, MCP endpoint or model processes.
fixture="$(mktemp -d)"
export XDG_CONFIG_HOME="$fixture/config"
export XDG_CACHE_HOME="$fixture/cache"
export XDG_DATA_HOME="$fixture/data"
if [[ $# -eq 0 ]]; then
  export LLAMA_LAUNCHER_DATA_DIR="$fixture/launcher-data"
  expected_data="$LLAMA_LAUNCHER_DATA_DIR"
else
  unset LLAMA_LAUNCHER_DATA_DIR
  expected_data="$(dirname "$(realpath "$1")")/data"
  [[ ! -e "$expected_data" ]] || { echo 'Use a fresh AppImage directory for this check' >&2; exit 1; }
fi
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
mkdir -p artifacts
for ((attempt=0;attempt<60;attempt++)); do
  if ! kill -0 "$launcher_pid" 2>/dev/null; then cat "$fixture/stderr"; exit 1; fi
  if grep -E 'panicked at|Unable to initialize|error while loading shared libraries' "$fixture/stderr"; then cat "$fixture/stderr"; exit 1; fi
  if [[ -s "$expected_data/working-settings.bin" ]] && xwininfo -root -tree | grep -q 'Llama Server Configuration Manager'; then
    cp "$fixture/stderr" artifacts/linux-window-stderr.log
    echo 'Native Linux window initialized and frontend saved private settings at the expected data location.'
    exit 0
  fi
  sleep 1
done
cp "$fixture/stderr" artifacts/linux-window-stderr.log
cat "$fixture/stderr"
echo 'Window/frontend did not initialize within 60 seconds' >&2
exit 1
