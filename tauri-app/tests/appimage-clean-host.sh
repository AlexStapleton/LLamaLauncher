#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
image=$(realpath "${1:?Supply the AppImage}")
test -f "$image"
mkdir -p artifacts
# A fresh newer userspace has no WebKitGTK 4.0. Do not mount the build host's
# libraries, models or credentials. Extraction mode also tests FUSE-free startup.
docker run --rm -v "$image:/input/LLamaLauncher.AppImage:ro" \
  -v "$(realpath tests/linux-window-smoke.sh):/input/window-smoke.sh:ro" \
  ubuntu:24.04 bash -euo pipefail -c '
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y --no-install-recommends xvfb xauth x11-utils dbus-x11 ca-certificates fonts-dejavu-core libgl1 libegl1 libgbm1 libasound2t64
    if ldconfig -p | grep -q libwebkit2gtk; then echo "Clean host unexpectedly has WebKitGTK" >&2; exit 1; fi
    mkdir -p /test
    cp /input/LLamaLauncher.AppImage /test/LLamaLauncher.AppImage
    chmod +x /test/LLamaLauncher.AppImage
    cd /test
    # Root in this disposable container needs the WebKit test-only sandbox override.
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
    dbus-run-session -- xvfb-run -a bash /input/window-smoke.sh /test/LLamaLauncher.AppImage
    echo "Clean Ubuntu 24.04 AppImage startup passed without system WebKitGTK or FUSE."
  ' 2>&1 | tee artifacts/appimage-clean-host.log
