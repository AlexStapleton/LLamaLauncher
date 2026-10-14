# How to build the Windows GUI — instructions for Pi

Use this procedure for the active Tauri application. It was successfully built on this desktop using the installed **MSVC** Rust toolchain. The default **GNU** toolchain on this machine fails with `dlltool` errors.

## 1. Work in the correct directory

Open PowerShell and run:

```powershell
Set-Location 'C:\Users\Ziggity\Documents\Pi\llam-server-rust\LLamaLauncher\tauri-app'
```

Close all running **Llama Server Configurator** windows before rebuilding or copying the executable. An open executable cannot be replaced on Windows. Coordinate with the user before closing an instance that may be managing a server.

Avoid concurrent edits or builds by multiple harnesses. Read the current files before changing them.

## 2. Check prerequisites

```powershell
node --version
npm --version
rustup toolchain list
cargo +stable-x86_64-pc-windows-msvc --version
```

This desktop already has the MSVC Rust toolchain, Microsoft C++ build tools, Windows resource compiler, and WebView2 Runtime. The GUI was launched successfully here. Do not reinstall prerequisites merely because the GNU build fails.

For a different machine, install the Windows MSVC Rust toolchain and Visual Studio Build Tools with the C++ desktop workload and Windows SDK. WebView2 Runtime is required to run the GUI.

If `node_modules` is absent, restore frontend dependencies with `npm ci`. This needs package-registry access. Do not manually edit installed dependencies.

## 3. Build the desktop executable

The recommended command is:

```powershell
.\build-windows.ps1
```

The script builds the frontend, builds the release executable with MSVC, checks command exit codes, and copies the executable into `desktop-app`.

If script execution is blocked, run these commands individually instead; no execution-policy change is needed:

```powershell
npm run build
```

Check that it exits successfully before continuing:

```powershell
cargo +stable-x86_64-pc-windows-msvc build --release --locked
```

Check that it exits successfully before copying:

```powershell
New-Item -ItemType Directory -Path '.\desktop-app' -Force | Out-Null
Copy-Item -LiteralPath '.\target\release\llama-server-configurator.exe' -Destination '.\desktop-app\llama-server-configurator.exe' -Force
```

For a build using already cached dependencies, add `--offline` to the Cargo command. If a dependency is missing from the cache, use the ordinary command with network access.

Never report success based only on an existing executable: it may be left from an older build. Check the build exit code and the executable's timestamp.

## 4. Verify the result

```powershell
npm test
cargo +stable-x86_64-pc-windows-msvc test --release --locked
Get-Item '.\desktop-app\llama-server-configurator.exe' | Format-List FullName,Length,LastWriteTime
```

Run `desktop-app\llama-server-configurator.exe` by double-clicking it. Confirm:

- A window titled **Llama Server Configuration Manager** opens.
- Start and Stop are always visible in the header, and the navigation tabs do not move when settings scroll. Stop is disabled when no app-owned process is running.
- Selecting a models folder populates the GGUF dropdown, including nested folders. Selecting a model updates the summary and launch command.
- Settings tabs respond, and Browse opens a native Windows file picker.
- All options reads the selected executable's `--help`; every option has a description and speed/VRAM/RAM guidance. The bundled snapshot has 257 groups and 414 flag names.
- Save JSON and Load JSON preserve the model selection and enabled overrides. Logs shows recent server output.

The existing desktop shortcut **Llama Server Configurator** points to this `desktop-app` executable. Refreshing that executable updates what the shortcut launches.

To test actual server operation, select a separate Windows `llama-server.exe`, a models directory, and a GGUF model inside that directory. Start and stop the server, and inspect its logs if it exits. Compiling or opening the GUI does not prove model inference works. Actual inference has not been verified in the current session.

## Troubleshooting observed failures

| Error | Meaning and action |
|---|---|
| `dlltool` missing or unable to create an import library | The command selected the Windows GNU toolchain. Use `cargo +stable-x86_64-pc-windows-msvc ...`. This desktop can build the program with MSVC. |
| `Could not resolve entry module "index.html"` | Check `vite.config.js`: its root is `./src/frontend`; use Vite's default HTML entry. The output must be `../../dist` relative to that root, which is `tauri-app/dist`. |
| Unknown `productName` or `tauri.app` field | Tauri 1 uses `package.productName`, `tauri.windows`, and `tauri.security`. Follow repository restrictions for correcting the configuration; preserve CSP. |
| Tauri features do not match the allowlist | Do not restore `api-all`, `dialog-all`, `fs-all`, `shell-open`, or `system-tray` indiscriminately. Current Cargo features are `wry` and `dialog`; the app uses its own Rust commands for browsing and JSON save/load. |
| `RC2175`, ICO not in 3.00 format | `icons/icon.ico` must be a real Windows ICO, not renamed image data or a placeholder. A working icon is already present. |
| `EPERM`, `Access is denied`, or unable to remove/copy an EXE | Check whether the configurator is running. A sandbox can also block Vite path resolution or WebView2's dependency build; request an approved build outside that sandbox. Do not label a sandbox error as a Rust code defect. |
| Cargo says the lockfile must change | For an intentional dependency/feature change, let Cargo regenerate it with a build without `--locked` (optionally `--offline` if dependencies are cached). Never hand-edit `Cargo.lock`. Return to `--locked` for repeatable builds. |
| `tauri_plugin_dialog`, `tauri_plugin_fs`, or `tauri_plugin_shell` unresolved | These plugin registrations were removed in the repaired Tauri 1 application. Do not reintroduce Tauri 2 plugin examples into this Tauri 1 project. |
| An executable exits immediately without a window | Check `src/main.rs`. It must initialize and run `tauri::Builder`; replacing it with an empty `fn main()` removes the GUI. |
| Undefined `DEFAULT_*`, `TauriAPI`, or `tauri` in JavaScript | Frontend defaults come from the `get_defaults` Rust command. Frontend/backend calls use Tauri 1's injected `window.__TAURI_INVOKE__`. Rust constants are not automatically JavaScript globals. |

## Rules for future changes

- Keep the real Tauri entry point, shared process state protected by a mutex, and serializable command results. A `Child` process belongs in Rust state, not in a frontend response.
- Launch the selected executable directly with `Command` arguments; do not concatenate a shell command or use `cmd` as the launcher.
- Run `npm run build` after frontend edits and before the Rust release build. Plain `cargo build` does not run the frontend build automatically.
- Do not edit `dist`, `target`, `node_modules`, or deprecated Python code by hand. Follow `AGENTS.md` restrictions on configuration changes.
- Use the existing source and valid icon. Do not replace broken functionality with placeholders to make compilation appear successful.
- This procedure does not require `cargo tauri` or installation of the latest Tauri CLI. The application remains on Tauri 1.

## Verified baseline

The MSVC release build and frontend build succeeded. Catalog tests cover every installed help entry and its impact guidance. Backend tests cover model discovery, argument passing, validation, configuration serialization, and process stopping. A headless browser harness (`tauri-app/tests/gui-smoke.mjs`, requiring Playwright) checks dropdown population, fixed navigation, always-visible Stop, option editing, search, persistence, start/stop states, logs and responsive layout. Its backend responses are mocked; Rust tests separately exercise the native backend logic.

The repository now includes compact grouped rows, a persistent light/dark toggle, and an MCP panel. Native MCP tests cover HTTP/SSE session negotiation and tool pagination, stdio discovery, protected profile round trips, and router launch arguments. The native window and file picker were checked in the initial build. Desktop UI automation was interrupted during the redesign, so do not describe the redesigned interface as fully tested by native UI interaction. Actual model inference has not been verified. A valid icon, desktop executable, and desktop shortcut are present. Keep these verification limits distinct from successful compilation.
