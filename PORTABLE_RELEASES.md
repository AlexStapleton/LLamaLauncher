# Portable releases

The public downloads are a Windows **setup `.exe`** and Linux **`.AppImage`**, plus SHA256 checksums. Windows setup includes the private x64 Fixed Version WebView2 runtime and licensed VC runtime DLLs; it installs for the current user, adds shortcuts, and preserves `data` on upgrade and uninstall. The default installation is `%LOCALAPPDATA%\Programs\LLamaLauncher`, a writable location so profiles remain beside the EXE. Close the application and stop its server before updating. Choose an existing portable folder in setup or copy its protected `data` folder while closed to retain it; Windows credentials require the same Windows account.

The AppImage bundles WebKitGTK and its process helpers; it does not require `libwebkit2gtk-4.0.so.37` from the host. Put it in a writable folder, `chmod +x` it and run it. Use `--appimage-extract-and-run` without FUSE. Its managed data is beside the original AppImage, outside the temporary mount. The supported baseline is Linux x86_64, glibc 2.39+, with a desktop session; broader distribution/GPU testing remains necessary.

llama.cpp and model weights are installed separately through Launch and Models, or linked from existing files. Distribution packages never contain developer data. Windows private records are account-bound; Linux uses private permissions. Both packaging paths retain offline settings documentation and dependency licenses.

Download the **Fixed Version x64** WebView2 CAB from https://developer.microsoft.com/en-us/microsoft-edge/webview2/ and accept Microsoft's redistribution terms. Extract it with Windows `expand.exe`, then provide its directory to the packaging script. Do not substitute the Evergreen installer or copy a system browser installation. Keep Microsoft notices and the runtime's license files in the package. Fixed WebView2 updates require a new launcher release.

Install NSIS 3 (makensis.exe). From `tauri-app` in PowerShell:

```powershell
./package-windows.ps1 -FixedWebView2Directory 'D:/BuildInputs/WebView2-fixed-x64'
```

For Ubuntu/Debian x64, build an AppImage on Ubuntu 24.04 with the Node version in `.nvmrc`, current stable Rust, Tauri 2 system development libraries and `xvfb` installed. The script invokes the locked Tauri 2 CLI through npx and creates a configuration override under ignored artifacts, preserves `tauri.conf.json`, and runs a virtual-display smoke check of the AppImage's FUSE-free startup. This asset's baseline is glibc 2.39 or newer; Debian 12/Ubuntu 22.04 are outside that target.

```bash
bash package-appimage.sh
```

Run the AppImage directly. If FUSE is unavailable, use `./LLamaLauncher.AppImage --appimage-extract-and-run` or `APPIMAGE_EXTRACT_AND_RUN=1`. Launch provides a Stable/Beta choice and validates the available official backend candidates, with a verified CPU fallback. Device enumeration is not a substitute for the supported hardware/driver test matrix.

Before release, test offline GUI startup, online engine installation, and a local CPU inference on a clean Windows x64 VM, Ubuntu 24.04 and Debian 13. Test a machine without installed WebView2, plus Linux without FUSE. Verify SHA256SUMS, upstream licenses, and dependency notices. A successful build is not proof that these clean-machine checks passed.

## Hugging Face browser sign-in

Register a public OAuth application without a client secret in Hugging Face settings. Configure read scopes (`openid profile read-repos gated-repos`) and paste the public client ID into Connections → Hugging Face account → Advanced: browser sign-in. Browser authorization uses device codes. The current implementation requests a new sign-in if access expires; it does not promise persistent refresh tokens. Read-token connection and public browsing work without an OAuth application registration. Never publish a personal token or add it to release configuration.

The Windows packager also copies the licensed Microsoft VC x64 runtime DLLs from Visual Studio, or from `-VCRuntimeDirectory`, into the portable folder. Keep their redistribution notices with your release.

## Current release policy

The 1.1.2 release pipeline runs from a main-branch push whose commit message begins `Release 1.1.2`. Both platform validation jobs must pass before creating tag `1.1.2` and publishing the setup EXE, AppImage and checksums. Windows tests installation, the real GUI using bundled WebView2, upgrade data preservation, and uninstall data preservation. Linux verifies WebKitGTK and WebKitWebProcess are inside the image, then starts it in a clean Ubuntu 24.04 container with no system WebKitGTK and no FUSE. Docker is required for this clean-host check. Windows uses pinned, checksum-verified and signature-checked Microsoft Fixed WebView2 155.0.4283.45 x64. See [1.1.2 release notes](RELEASE_NOTES_1.1.2.md). The existing 1.1 release/tag remains unchanged.

Local Windows packages are unsigned beta builds until the platform/hardware acceptance gates in `PRODUCTION_READINESS_REVIEW.md` pass. A checksum verifies downloaded bytes; it does not establish publisher identity. Production Windows releases require an Authenticode certificate supplied through the release environment and signature verification before packaging. No signing key or certificate is stored in this repository. Linux distributions require a corresponding verified native/AppImage test report; preparing a workflow does not close that gate.

The packagers generate `THIRD_PARTY_NOTICES.txt` from locked dependency metadata and accompanying license texts. Keep this file, the application license, and Microsoft redistribution notices with the distributed binaries. Review the generated notice inventory when changing dependencies. The compatible glib source backport retains its upstream licenses and provenance; the source verification script must pass before packaging.

Updates replace the application and private WebView2 runtime while preserving `data`. First copy that folder as an account-protected backup. Fixed WebView2 servicing requires rebuilding and testing a new package with the pinned official runtime and its checksum. Do not reuse a previously tested package label for changed binaries.

## Saved-data recovery

Launch → Saved data & recovery shows working-save failures and damaged records. Retry saving retains the pending edits. Restore backup or Reset this record is an explicit action, preserves the original protected bytes, and restarts the application. Backups remain protected by the same Windows account (Linux: private permissions). Folder portability does not make Windows credentials readable by a different account.

Server stdout and stderr each retain three files of at most 10 MiB, for a combined 60 MiB disk cap. Known credentials are scrubbed before writing. Custom arguments remain private in profiles; shareable JSON export deliberately omits custom executable/MCP arguments and authentication fields.

The dependency pass after 1.1.2 refreshes Node, NSIS, Fixed WebView2 and app-local VC DLLs; see [DEPENDENCY_REVIEW.md](DEPENDENCY_REVIEW.md) for exact pins and remaining native-library gaps. These source changes do not replace the published 1.1.2 assets. Ordinary main-branch builds now validate installers/AppImages, including clean Ubuntu 24.04 and Debian 13 startup, before a future release. The verified Microsoft runtime inputs are in tauri-app/runtime-inputs.json; Windows release packaging extracts current app-local VC DLLs from the signed bundle without installing a system runtime.
