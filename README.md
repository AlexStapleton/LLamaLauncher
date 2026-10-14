# LLamaLauncher

A desktop launcher and configurator for llama.cpp, built with Rust/Tauri and a web frontend. Windows is verified locally; Linux support uses native process groups and a portable frontend, with a Windows/Linux build workflow prepared for GitHub Actions.

## Features

- GGUF discovery in folders and subfolders; single-model and on-demand router modes.
- All options from the selected llama-server executable's help output.
- Compact settings grouped by category, with expandable descriptions and speed/VRAM/RAM guidance.
- Persistent light/dark themes and pinned server controls/navigation.
- MCP connection status, connection testing, tool discovery/search, and schema inspection. Connection tests do not execute tools.
- Streamable HTTP MCP with optional bearer authentication, plus local stdio MCP executables. An included HTTP-to-stdio bridge supplies remote MCP servers to llama-server's native stdio integration.
- Windows DPAPI-protected connection profiles. Linux stores profiles in user-only configuration files (not encrypted).
- Owned process-tree cleanup, JSON configuration import/export, and recent server logs.
- Health-based loading/ready status, bounded MCP framing, cancellable model scans and lazy settings/help rendering.
- Versioned configuration with legacy import migration and generated TypeScript native contracts.
- Named launch profiles with load/quick launch, update, rename, duplicate and delete; each saves the model, runtime and all settings in private native storage.

See [optimization implementation and measurements](OPTIMIZATION_IMPLEMENTATION.md) for the changes, measured tradeoffs and remaining Linux verification.

## Build on Windows

Install Node.js 22.12 or later, the Windows MSVC Rust toolchain, Visual Studio C++ Build Tools/Windows SDK, and WebView2 Runtime. The frontend uses Vite 8 and TypeScript; the runtime remains Tauri 1.x. From this repository:

```powershell
cd tauri-app
npm ci
.\build-windows.ps1
```

Run `tauri-app/desktop-app/llama-server-configurator.exe`. The frontend is embedded; it needs no development server at runtime. The build script explicitly selects MSVC to avoid missing-MinGW/dlltool failures.

## Build on Linux

For an Ubuntu 22.04-based environment, install the Tauri 1 system dependencies:

```sh
sudo apt-get install build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev patchelf
cd tauri-app
npm ci
bash build-linux.sh
```

Use the native Linux Rust toolchain. The result is `tauri-app/desktop-app/llama-server-configurator`. A desktop display session is required to run the GUI. Linux compilation/runtime has not been verified on the current Windows desktop.

## MCP and router mode

Open MCP, enter an endpoint ending in `/mcp` (or a local executable and argument array), then select Test connection or Connect. The app initializes MCP and reads every page of `tools/list`. Connect enables router mode and includes the profile on the next server start. A running server must be stopped and restarted to apply changed launch settings.

Router mode omits a fixed `--model`, uses the chosen models directory or llama.cpp's model cache, and loads models on demand. The selected GGUF is preserved for switching back to single-model mode. HTTP MCP profiles are passed through the included stdio bridge; credentials are not embedded in the server command preview.

Profile storage is separate from the repository. Never commit tokens or private connection files. Tokens and environment values are excluded from browser local storage. Explicitly saved configuration JSON includes the values you chose to export, so handle it as a private file.

## Launch profile manager

Configure the executable, model (or router folder), API settings and runtime options. Open **Profiles**, give the setup a custom name and choose **Save current as new**. All overrides, extra arguments and MCP settings are included. Use the fixed **Launch profile** dropdown to load a saved setup, then **Launch saved** to start it. **Start server** launches the current working settings, including unsaved edits.

The manager can update a saved profile from current settings, rename it, duplicate its saved settings under a new name, or delete it. Switching prompts before replacing unsaved edits; updating and deleting require confirmation. Profiles persist across app restarts. Deleting a profile keeps the current working settings and never deletes model/executable files. A running server must be stopped before another saved profile can launch.

Saved model paths are retained if the file is missing; profile loading never silently replaces them with the first available model. File existence and executable/model validation still happen on launch. Profiles save paths and settings, not copies of model weights or executables. Paths may need adjustment on a different computer or operating system.

The library lives beside the native MCP profile as `launch-profiles.bin`, outside this repository: Windows DPAPI-protected storage or Linux account-only directory/file permissions. Full credentials are restored only when loading a profile; list summaries contain no credentials. Library writes use atomic replacement, duplicate names are rejected, and malformed libraries produce an error instead of being silently reset. Linux profile files are private but are not encrypted.

Verified on the current Windows desktop: the Docker AI MCP Gateway authenticated successfully and returned 49 tools through both the native client and the HTTP bridge; a llama-server router started and passed its health/model-list checks with an empty model directory. No tools were executed and no model inference was performed.

References: [llama.cpp server/router/MCP documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md), [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [MCP tool discovery](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

## Development and verification

```sh
cd tauri-app
npm test
npm run build
cargo test --release --locked
cargo build --release --locked
```

On this Windows desktop use `cargo +stable-x86_64-pc-windows-msvc` in those commands. The optional `tests/gui-smoke.mjs` uses Playwright for browser-level interface checks with mocked backend responses; Rust tests separately exercise HTTP JSON/SSE MCP, stdio MCP, protected profile storage, model scanning, router arguments, and process cleanup.

Source lives in `tauri-app/src/main.rs`, `config.rs`, `catalog.rs`, `server.rs`, `framing.rs`, `mcp.rs`, `storage.rs`, `process_tree.rs`, and `src/frontend/`. Configuration validation stays in main.rs. Run `npm run types:generate` after changing a native DTO and commit its generated interface; `npm run typecheck` detects drift. See [AGENTS.md](AGENTS.md) and [the Windows build guide](HOW_TO_BUILD_WINDOWS.md). Generated binaries, dependency directories, screenshots, caches and credentials are excluded from version control.
