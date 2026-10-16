# Portable releases

Current source builds produce **one portable Windows `.exe`** and **one Linux `.AppImage`**, plus optional SHA256 checksum files. Earlier published 1.1.2 assets remain unchanged: that Windows download is a setup installer, not the new portable container.

## Windows

Put `LLamaLauncher-<version>-Windows-x86_64.exe` in a writable folder and double-click it. The GUI opens directly. It does not install a system runtime, register an uninstaller, or create desktop/Start menu shortcuts. You can create a shortcut to the downloaded EXE yourself.

The single download embeds the application, private x64 Fixed WebView2, licensed VC DLLs, dependency notices and offline settings documentation. Windows requires the runtime's individual files on disk: the container extracts them into a unique NSIS-owned temporary folder for the session, waits for the GUI, and cleans up on normal exit. Extraction adds startup time and temporary disk usage. A forced termination or power loss can leave temporary files for Windows to clean later. Runtime files never contain profiles/models.

Profiles, settings, logs, installed llama.cpp engines and default model downloads remain in **data beside the original downloaded EXE**, not beside its temporary copy or in the current working directory. Saved-data recovery restarts the GUI through the container while retaining its runtime. Keep the EXE and data together when moving them. Close the app and stop its server before replacing the EXE to update. No uninstall is needed: remove the EXE; keep data if you want your settings/models. Windows private records remain protected by the same Windows account.

Build the native release first, then run from `tauri-app`:

```powershell
./release-windows.ps1
```

The recipe fetches official Microsoft inputs pinned in `runtime-inputs.json`, checks hashes, signatures, exact versions and x64 architecture, extracts app-local VC DLLs without installing a system runtime, and compiles the container with NSIS 3.13. It produces `artifacts/release/LLamaLauncher-<version>-Windows-x86_64.exe` and its checksum. It then runs `tests/windows-portable-smoke.ps1` against a fresh copy: real GUI/embedded WebView2, adjacent private settings, rename/relaunch preservation, normal temporary cleanup and no installer registration/desktop shortcut changes. These checks are distinct from broad clean-Windows/GPU coverage.

For manually prepared, verified redistribution inputs:

```powershell
./package-windows.ps1 -FixedWebView2Directory 'D:/BuildInputs/WebView2-fixed-x64' -VCRuntimeDirectory 'D:/BuildInputs/VC-x64' -SkipBuild -ExecutablePath 'target/release/llama-server-configurator.exe'
```

Keep Microsoft's redistribution notices and the runtime's license files in the embedded stage. Do not copy an installed system browser. Fixed WebView2 servicing requires a new verified package. Models, engines, private data and test fixtures are excluded. `package-portable.ps1 -StageOnly` is the internal staging helper; its directory/ZIP output is not the public Windows download.

## Linux

Build on Debian 13 using the Node version in `.nvmrc`, current stable Rust, Tauri 2 development libraries and xvfb:

```bash
bash release-linux.sh
```

The locked Tauri CLI uses an ignored configuration override, preserving `tauri.conf.json`. The AppImage bundles WebKitGTK 4.1/libsoup 3 and their process helpers, including support libraries identified by clean-host tests. Native versions, copyrights, dependency notices and the settings guide are included. Docker is required for clean-host verification on Debian 13 and Ubuntu 26.04 without system WebKitGTK/FUSE. Root-only WebKit sandbox overrides are confined to disposable test processes, not the production launcher.

Mark the AppImage executable and run it. Without FUSE, use `./LLamaLauncher.AppImage --appimage-extract-and-run` or `APPIMAGE_EXTRACT_AND_RUN=1`. Current source uses XDG config/data/state/cache directories, so it does not need to write beside the image or inside its mount. New GGUF downloads default to `LLamaLauncher/models` in the configured XDG Downloads folder (application data if no Downloads directory is configured); the Models folder remains an explicit override. Engines install under `$XDG_DATA_HOME/llama-launcher/runtimes`. Existing saved paths and large files are retained, and bounded old metadata/backups are copied once without overwriting existing XDG records. See [README storage details](README.md#navigation-and-engine-setup). New source builds require Linux x86_64 and glibc 2.41 or newer; Ubuntu 24.04, Debian 12, ARM and musl are outside that target. Published 1.1.2 keeps its original glibc 2.39 baseline and adjacent-data behavior. Container startup does not establish every desktop/GPU/driver combination.

## Release and verification policy

Ordinary main-branch CI builds both packages and runs their native checks before a future release. After Windows smoke tests and Linux clean-host checks pass, the artifacts named `LLamaLauncher-<version>-Windows-x86_64.exe` and `LLamaLauncher-<version>-Linux-x86_64.AppImage` are uploaded directly with `archive: false`: downloading these individual artifacts does not require unzipping. GitHub sign-in and artifact retention limits apply. Separate internal release/checksum and diagnostic archives remain for automation; choose the artifacts with the actual executable/AppImage filenames to test the application.

The release publisher verifies the exact expected executable/AppImage and checksums. Existing published tags/assets must not be replaced with different bytes; a new version and reviewed release notes are required to publish these changes. The historical 1.1.2 trigger cannot republish its changed source because its existing release targets another commit.

Validated on 2026-10-16: [CI run 38971024056](https://github.com/AlexStapleton/LLamaLauncher/actions/runs/38971024056), source `ddbfcd8421dab37aa4c6d85c9e9d3f1caa26409f`, passed both platform source/backend/interface checks, the complete Windows portable smoke test, and clean Debian 13/Ubuntu 26.04 AppImage startup. The Windows test covers embedded-runtime GUI startup, original-EXE data location, rename/relaunch preservation, recovery restart protocol, temporary cleanup and no install registration changes. A separate local native GUI check exercised protected-record recovery, the container restart exit code and a ready replacement GUI with preserved originals. Both final single-file artifacts were uploaded without ZIP wrapping. These are unreleased test builds retaining the source version 1.1.2; they do not replace the published 1.1.2 installers. Broad clean-Windows/GPU/desktop coverage remains separate.

The dependency pass after 1.1.2 refreshes Node, NSIS, Fixed WebView2 and app-local VC DLLs, and moves Linux builds to Debian 13; [DEPENDENCY_REVIEW.md](DEPENDENCY_REVIEW.md) records exact pins, executed checks and remaining constraints. `WINDOWS_PACKAGE_VERSIONS.json` records Microsoft runtime versions/signers/hashes. Linux inventories record native package versions. SHA256 checksums verify bytes, not publisher identity. Builds remain unsigned beta packages until the acceptance gates in [PRODUCTION_READINESS_REVIEW.md](PRODUCTION_READINESS_REVIEW.md) pass. No certificate or private key belongs in the repository.

Before production publication, also exercise clean Windows startup without installed WebView2, online engine installation, real inference, desktop integrations and relevant hardware/driver combinations. Local CPU/native integration and clean container startup evidence should be described accurately, without expanding it into untested coverage. Never package developer data or models.

## Hugging Face browser sign-in

Register a public OAuth application without a client secret in Hugging Face settings. Configure read scopes (`openid profile read-repos gated-repos`) and paste the public client ID into Connections → Hugging Face account → Advanced: browser sign-in. Browser authorization uses device codes. The current implementation requests a new sign-in if access expires; it does not promise persistent refresh tokens. Read-token connection and public browsing work without an OAuth application registration. Never publish a personal token or add it to release configuration.

## Saved-data recovery

Launch → Saved data & recovery shows working-save failures and damaged records. Retry saving retains pending edits. Restore backup or Reset this record preserves the original protected bytes and restarts the GUI. Backups remain protected by the same Windows account (Linux: private permissions). Folder portability does not make Windows credentials readable by another account.

Server stdout and stderr each retain three files of at most 10 MiB, for a combined 60 MiB disk cap. Known credentials are scrubbed before writing. Custom arguments remain private in profiles; shareable JSON export deliberately omits custom executable/MCP arguments and authentication fields.
