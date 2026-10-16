#!/usr/bin/env bash
set -euo pipefail
# Fresh profile: no real credentials, MCP endpoint or model processes.
fixture="$(mktemp -d)"
export XDG_CONFIG_HOME="$fixture/config"
export XDG_CACHE_HOME="$fixture/cache"
export XDG_DATA_HOME="$fixture/data"
export XDG_STATE_HOME="$fixture/state"
unset LLAMA_LAUNCHER_DATA_DIR
expected_data="$XDG_CONFIG_HOME/llama-launcher"
mkdir -p "$XDG_CONFIG_HOME"
printf 'XDG_DOWNLOAD_DIR="%s"\n' "$fixture/Model downloads" > "$XDG_CONFIG_HOME/user-dirs.dirs"
if [[ $# -gt 0 ]]; then
  adjacent_data="$(dirname "$(realpath "$1")")/data"
  [[ ! -e "$adjacent_data" ]] || { echo 'Use a fresh AppImage directory for this check' >&2; exit 1; }
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
    grep -Fq 'llama-launcher-relative:/models' "$expected_data/working-settings.bin"
    test -d "$XDG_DATA_HOME/llama-launcher/models"
    test ! -e "$fixture/Model downloads/LLamaLauncher/models"
    test -d "$XDG_DATA_HOME/llama-launcher"
    test -d "$XDG_STATE_HOME/llama-launcher"
    test -d "$XDG_CACHE_HOME/llama-launcher"
    [[ $(stat -c %a "$expected_data/working-settings.bin") == 600 ]]
    [[ $(stat -c %a "$expected_data") == 700 ]]
    if [[ $# -gt 0 ]]; then test ! -e "$adjacent_data"; fi
    cp "$fixture/stderr" artifacts/linux-window-stderr.log
    echo 'Native Linux window initialized, saved private XDG configuration, and used application data for models without adjacent AppImage data.'
    exit 0
  fi
  sleep 1
done
cp "$fixture/stderr" artifacts/linux-window-stderr.log
cat "$fixture/stderr"
echo 'Window/frontend did not initialize within 60 seconds' >&2
exit 1
