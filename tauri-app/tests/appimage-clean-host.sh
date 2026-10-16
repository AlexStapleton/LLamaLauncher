#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
image=$(realpath "${1:?Supply the AppImage}")
test -f "$image"
mkdir -p artifacts
: > artifacts/appimage-clean-host.log
# Fresh userspaces have no WebKitGTK installed. Do not mount the build host's
# libraries, models or credentials. Extraction mode also tests FUSE-free startup.
for distro in ubuntu:24.04 debian:13; do
docker run --rm -v "$image:/input/LLamaLauncher.AppImage:ro" \
  -v "$(realpath tests/linux-window-smoke.sh):/input/window-smoke.sh:ro" \
  "$distro" bash -euo pipefail -c '
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y --no-install-recommends xvfb xauth x11-utils dbus-x11 ca-certificates fonts-dejavu-core libgl1 libegl1 libgbm1 libasound2t64
    if ldconfig -p | grep -q libwebkit2gtk; then echo "Clean host unexpectedly has WebKitGTK" >&2; exit 1; fi
    mkdir -p /test
    cp /input/LLamaLauncher.AppImage /test/LLamaLauncher.AppImage
    chmod +x /test/LLamaLauncher.AppImage
    cd /test
    mkdir dependency-check
    (cd dependency-check && /test/LLamaLauncher.AppImage --appimage-extract > extraction.log)
    appdir=/test/dependency-check/squashfs-root
    paths=$(find "$appdir/usr" -type d \( -name lib -o -name x86_64-linux-gnu \) -print | paste -sd:)
    missing=0
    while IFS= read -r binary; do
      report=$(LD_LIBRARY_PATH="$paths" ldd "$binary" 2>&1 || true)
      if grep -q "not found" <<< "$report"; then echo "$binary"; echo "$report"; missing=1; fi
    done < <(find "$appdir/usr" -type f \( -name llama-server-configurator -o -name WebKitWebProcess -o -name WebKitNetworkProcess -o -name WebKitGPUProcess \))
    [[ $missing -eq 0 ]] || { echo "AppImage has unresolved clean-host dependencies" >&2; exit 1; }
    # Root in this disposable container needs the WebKit test-only sandbox override.
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
    dbus-run-session -- xvfb-run -a bash /input/window-smoke.sh /test/LLamaLauncher.AppImage
    . /etc/os-release
    echo "Clean $PRETTY_NAME AppImage startup passed without system WebKitGTK or FUSE."
  ' 2>&1 | tee -a artifacts/appimage-clean-host.log
done
