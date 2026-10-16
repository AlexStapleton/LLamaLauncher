# LLamaLauncher

A desktop launcher and configurator for llama.cpp, built with Rust/Tauri and a web frontend. Windows and Linux builds passed native GitHub Actions validation for 1.1. Release 1.1.2 switches downloads to a Windows setup executable and Linux AppImage.

Version 1.1.2 packaging notes and download instructions are in [RELEASE_NOTES_1.1.2.md](RELEASE_NOTES_1.1.2.md). Publication is gated on both platform jobs; packages include checksums and dependency notices. Windows packages remain unsigned beta builds.

## Features

- GGUF discovery in folders and subfolders; single-model and on-demand router modes.
- All options from the selected llama-server executable's help output.
- Compact settings grouped by purpose, with reviewed explanations, direction of change, magnitude examples and speed/VRAM/RAM guidance linked to official llama.cpp sources. See the [complete settings guide](LLAMA_SETTINGS_GUIDE.md).
- Persistent light/dark themes and pinned server controls/navigation.
- MCP connection status, connection testing, tool discovery/search, and schema inspection. Connection tests do not execute tools.
- Streamable HTTP MCP with optional bearer authentication, plus local stdio MCP executables. An included HTTP-to-stdio bridge supplies remote MCP servers to llama-server's native stdio integration.
- Private records in `data` beside the executable: Windows DPAPI protection; Linux account-only permissions (not encryption).
- Owned process-tree cleanup, JSON configuration import/export, and recent server logs.
- Health-based loading/ready status, bounded MCP framing, cancellable model scans and lazy settings/help rendering.
- Versioned configuration with legacy import migration and generated TypeScript native contracts.
- Named launch profiles with load/quick launch, update, rename, duplicate and delete; each saves the model, runtime and all settings in private native storage.
- Hardware detection and preflight, with GGUF metadata and approximate RAM/VRAM estimates using the selected context and cache precision.
- Live dashboard with a saved 1–60 second polling interval and a complete monitoring-off switch. Server readiness and Start/Stop remain independent.
- Streaming inference test, cancellation, timings, and access to the server's existing browser chat.
- Hugging Face GGUF search, read-token or device-code sign-in, gated-access guidance, companion/shard selection and verified resumable downloads. Browser sign-in needs a registered public OAuth client ID.
- Side-by-side official runtime installation, explicit switching/rollback, and CUDA dependency handling; imported runtimes remain supported.
- Router model state, load/unload, server events with polling fallback, and saved-profile benchmark comparison/export. Benchmark runs disable tools/MCP and require the active server to be stopped.
- Protected backups and explicit saved-data recovery, save retry, and close-time working-settings flushing.
- Bounded rotating logs, a two-worker download queue, aggregated progress, and recoverable runtime installation receipts.
- Router monitoring includes model workers/helpers; benchmark memory figures are sampled peaks.

See [portable release packaging](PORTABLE_RELEASES.md) for the Windows setup `.exe` and Linux `.AppImage`. Models are not bundled. Windows packages contain a private fixed WebView2 runtime; llama.cpp is selected or downloaded during Launch setup. Current source builds target Debian 13/glibc 2.41 or newer, with clean Debian 13 and Ubuntu 26.04 AppImage startup checks without system WebKitGTK. Published 1.1.2 assets retain their original baseline.

See [optimization implementation and measurements](OPTIMIZATION_IMPLEMENTATION.md) for the changes, measured tradeoffs and remaining Linux verification.

For the current fixes and validation, read the [production readiness review](PRODUCTION_READINESS_REVIEW.md) and [changelog](CHANGELOG.md). The earlier optimization reports are historical. Packages remain beta builds; the broader GPU/driver matrix, clean Windows deployment and extended soak testing remain release gates. The original 1.1 workflow passed on Windows and Linux; historical review entries retain their original scope.

## Download and run

Use the latest release's **Windows-x86_64-Setup.exe** on Windows. It installs for the current account, adds desktop/Start menu shortcuts, and includes WebView2 and VC runtime DLLs. The default folder is `%LOCALAPPDATA%\Programs\LLamaLauncher`; profiles and downloaded engines/models remain in its `data` folder. Updates and uninstall preserve that folder. To use an existing portable installation, close it first and choose its folder in setup, or copy its protected `data` folder to the new installation while closed (same Windows account).

On Linux, download **Linux-x86_64.AppImage**, place it in a writable folder and run:

```sh
chmod +x LLamaLauncher-1.1.2-Linux-x86_64.AppImage
./LLamaLauncher-1.1.2-Linux-x86_64.AppImage
```

If FUSE is unavailable, add `--appimage-extract-and-run`. WebKitGTK is bundled; no host WebKitGTK 4.0 installation is required. Linux x86_64 with glibc 2.39 or newer and a desktop display is required; this is not an ARM or Alpine/musl build. Keep the adjacent `data` folder when replacing the AppImage. The old 1.1 Linux ZIP was a raw binary requiring system WebKitGTK; use the new AppImage instead.

## Build on Windows

Use the Node.js version in `.nvmrc` (currently stable 26.12.0), the Windows MSVC Rust toolchain, Visual Studio C++ Build Tools/Windows SDK, and WebView2 Runtime. Node is a build/test tool and is not needed to run the desktop app. The frontend uses Vite 8 and TypeScript; the runtime uses Tauri 2.x. From this repository:

```powershell
cd tauri-app
npm ci
.\build-windows.ps1
```

Run `tauri-app/desktop-app/llama-server-configurator.exe`. The frontend is embedded; it needs no development server at runtime. The build script explicitly selects MSVC to avoid missing-MinGW/dlltool failures.

## Build on Linux

For current source builds, use Debian 13 and install the Tauri 2 system dependencies:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config ca-certificates curl wget file zstd libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev libgpg-error0 libcom-err2 patchelf xvfb xauth x11-utils
cd tauri-app
npm ci
bash build-linux.sh
```

Use the native Linux Rust toolchain. The result is `tauri-app/desktop-app/llama-server-configurator`. A desktop display session is required to run the GUI. This raw developer binary needs the system libraries above. For the distributable AppImage, run `bash package-appimage.sh` on Linux (the script uses the locked Tauri 2 CLI through npx). Native Linux CI passed for 1.1; 1.1.2 adds AppImage and clean-host packaging checks.

## MCP and router mode

Open Connections, enter an endpoint ending in `/mcp` (or a local executable and argument array), then select Test connection or Connect. The app initializes MCP and reads every page of `tools/list`. Connect enables router mode and includes the profile on the next server start. A running server must be stopped and restarted to apply changed launch settings.

Router mode omits a fixed `--model`, uses the chosen models directory or llama.cpp's model cache, and loads models on demand. The selected GGUF is preserved for switching back to single-model mode. HTTP MCP profiles are passed through the included stdio bridge; credentials are not embedded in the server command preview.

Private records live beside the executable and are excluded from version control. Never commit tokens or connection files. Named profiles retain credentials privately; shareable JSON export omits authentication fields, custom executable/MCP arguments, and URL authentication/query data. Known credentials are scrubbed from previews and logs; arbitrary unmarked free-text secrets cannot be identified reliably.

## Local network access

In **Launch → Server address & port**, choose **Local network** to listen on all IPv4 interfaces (`0.0.0.0`), or enter this computer's specific local IP to bind only that interface. Set the port and restart the server. Other devices connect to `http://<this computer's local IP>:<port>`; `0.0.0.0` is a listen address, not the address clients enter. **Only this computer** restores `127.0.0.1`. These settings are included in saved profiles. Allow llama-server on the Windows private network if the firewall prompts. API authentication and other server options remain under **Settings → API**.

## Launch profile manager

Configure the executable, model (or router folder), API settings and runtime options. Open **Profiles**, give the setup a custom name and choose **Save current as new**. All overrides, extra arguments and MCP settings are included. Use the fixed **Launch profile** dropdown to load a saved setup, then **Launch saved** to start it. **Start server** launches the current working settings, including unsaved edits.

The manager can update a saved profile from current settings, rename it, duplicate its saved settings under a new name, or delete it. Switching prompts before replacing unsaved edits; updating and deleting require confirmation. Profiles persist across app restarts. Deleting a profile keeps the current working settings and never deletes model/executable files. A running server must be stopped before another saved profile can launch.

Saved model paths are retained if the file is missing; profile loading never silently replaces them with the first available model. File existence and executable/model validation still happen on launch. Profiles save paths and settings, not copies of model weights or executables. Paths may need adjustment on a different computer or operating system.

The library lives in `data/launch-profiles.bin` beside the launcher EXE: Windows DPAPI-protected storage or Linux account-only directory/file permissions. Full credentials are restored only when loading a profile; list summaries contain no credentials. Library writes use atomic replacement, duplicate names are rejected, and malformed libraries produce an error instead of being silently reset. Linux profile files are private but are not encrypted.

Windows verification includes real CPU inference, verified public-model downloads, runtime installation/removal and receipt-based retry after an injected registry-write failure, benchmark runs, monitoring off, two loaded router models, and portable-folder relocation. MCP fixtures initialize/list tools without executing them. Historical gateway discovery checks are described in the earlier optimization report.

References: [llama.cpp server/router/MCP documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md), [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [MCP tool discovery](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

## Development and verification

```sh
cd tauri-app
npm test
node tests/verify-glib-backport.mjs
npm run build
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --release --locked
cargo build --release --locked
```

On this Windows desktop use `cargo +stable-x86_64-pc-windows-msvc` in those commands. The optional `tests/gui-smoke.mjs` uses Playwright for browser-level interface checks with mocked backend responses; Rust tests separately exercise HTTP JSON/SSE MCP, stdio MCP, protected profile storage, model scanning, router arguments, and process cleanup.

Source lives in `tauri-app/src/main.rs`, `config.rs`, `catalog.rs`, `server.rs`, `framing.rs`, `mcp.rs`, `storage.rs`, `process_tree.rs`, and `src/frontend/`. Configuration validation stays in main.rs. Run `npm run types:generate` after changing a native DTO and commit its generated interface; `npm run typecheck` detects drift. See [AGENTS.md](AGENTS.md) and [the Windows build guide](HOW_TO_BUILD_WINDOWS.md). Generated binaries, dependency directories, screenshots, caches and credentials are excluded from version control.

The main UI/MCP modules are strict TypeScript (`app.ts`, `mcp-ui.ts`); native job results are validated at the frontend boundary. Windows local checks passed 58 Rust tests and 11 frontend tests, plus browser event regressions and native integration. Three Rust fixture entry points are launched by their parent tests. Optional native checks are `tests/native-features.mjs`, `tests/portable-storage.mjs` and `tests/portable-package-smoke.mjs`; their required inputs and isolation are documented in [AGENTS.md](AGENTS.md). Linux uses a compatible [glib source backport](vendor/glib-0.18.5/BACKPORT.md); its optimized regression must run on Linux before release.

## Navigation and engine setup

Six main sections organize the application: **Launch** for llama.cpp installation, server mode and address/port; **Models** for the models folder, GGUF selection, Hugging Face discovery, inspection, router models, adapters and media; **Settings** for Performance, GPU & memory, Context & cache, Sampling, Chat & output, API, Advanced and All options; **Profiles** for saved launches and benchmark comparisons; **Activity** for monitoring, inference tests, logs and background tasks; and **Connections** for Hugging Face accounts and MCP. Native logging controls live beside Logs, and native tool/MCP overrides beside Connections. Speculative decoding controls, including the draft model's CPU/GPU/cache settings, stay together in Advanced.

Each option's **Help & resource impact** explains what it does, what increasing/decreasing or enabling/disabling changes, how large the effect can be under stated assumptions, and separate speed/VRAM/RAM effects. For example, doubling dense context roughly doubles KV capacity, while changing f16 cache components to q8_0 uses about 53% of their raw payload; neither predicts a fixed speedup or total application memory saving. The selected runtime's original help and defaults remain visible. New version-specific flags remain available with an explicit unreviewed label. All 257 bundled option groups are covered in the [settings reference](LLAMA_SETTINGS_GUIDE.md).

The portable launcher includes its GUI dependencies. It does not bundle a duplicate llama.cpp engine. In Launch, browse to an existing `llama-server` or expand **Download or update llama.cpp from GitHub**, choose **Stable** or **Beta**, then press **Download llama.cpp**. The launcher validates candidate executables and actual GPU device enumeration, prefers a CUDA candidate on NVIDIA when available, tries Vulkan, and falls back to a verified CPU build. Device enumeration does not guarantee model performance or every driver/workload combination. Stable follows GitHub's latest non-prerelease and, when needed, its published `nightly-tag.txt` mapping. Beta follows the newest prerelease. Stable is never silently replaced by Beta. Installation selects the engine for current working settings; existing saved profiles keep their version.

Choose your GGUF in **Models**, then press **Use for single-model launch** to return to Launch. The picker stays visible in router mode. Single-model mode turns off MCP for that launch; router mode loads advertised models on demand. Launch shows the selected model, missing setup requirements and startup failures.

Saved profiles, working settings, accounts, monitoring preferences, downloaded engines, default downloaded models, logs and WebView data live in a **data** folder beside the EXE (or beside the original AppImage on Linux). Copy the whole launcher folder to preserve it. Managed paths inside data are stored relative to it, so moving that folder preserves references. Existing external models and imported executables retain their paths. Windows private records remain bound to the Windows account through DPAPI; Linux files use account-only permissions. First startup copies existing native profile/account records from the old per-user directory without deleting them. Existing externally located runtime/model files are not moved. The test-only `LLAMA_LAUNCHER_DATA_DIR` override isolates integration fixtures.

Public Hugging Face searches and downloads need no account. For gated/private repositories, connect a read token in Connections; **Get a read token** opens Hugging Face account settings. Browser sign-in remains under advanced account settings and is disabled until a registered public OAuth client ID is entered. GGUF files appear next to search results; loading an optional model card no longer delays file selection. Action failures appear beside the relevant controls.

Launch → **Saved data & recovery** shows save failures and damaged records. Retry retains pending edits; restore/reset preserves original protected bytes and restarts the app. Logs retain up to 60 MiB across stdout/stderr and backups. Keep `data` when updating the launcher. Build/package outputs are generated locally or obtained from GitHub Actions artifacts; ZIPs, models, private data and test outputs are not committed to this repository.

Dependency currency, upgraded libraries, retained framework/native constraints, and validation evidence are recorded in [DEPENDENCY_REVIEW.md](DEPENDENCY_REVIEW.md). The full locked graph is in [DEPENDENCY_INVENTORY.md](DEPENDENCY_INVENTORY.md).
