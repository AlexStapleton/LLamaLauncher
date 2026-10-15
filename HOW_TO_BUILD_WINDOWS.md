# Build the Windows application

Use the Rust/Tauri application in `tauri-app`. These instructions also apply to Pi and other build harnesses. Read [AGENTS.md](AGENTS.md) before changing source.

## Prerequisites

- Node.js 22.12 or later and npm.
- Rust 1.89 or later, using the stable Windows MSVC toolchain.
- Visual Studio C++ Build Tools, the desktop C++ workload and Windows SDK.
- WebView2 Runtime for an ordinary executable build, or the official Fixed Version x64 runtime for a portable ZIP.

Check the selected toolchain:

```powershell
node --version
rustup toolchain list
cargo +stable-x86_64-pc-windows-msvc --version
```

The reviewed desktop's default GNU toolchain fails with missing MinGW/dlltool dependencies. Selecting MSVC resolves that environment issue; reinstalling MinGW is not required for this build.

## Build

From the repository root:

```powershell
Set-Location .\tauri-app
npm ci
.\build-windows.ps1
```

The script checks frontend tests, builds embedded frontend assets, checks Rust, builds with MSVC and copies the EXE to `desktop-app/llama-server-configurator.exe`. No Node server is needed at runtime. Close a running copy before replacing its EXE; coordinate first if it owns a server. Do not run competing source edits/builds from multiple harnesses.

If script execution is restricted, use the commands directly:

```powershell
npm test
npm run build
cargo +stable-x86_64-pc-windows-msvc check --locked
cargo +stable-x86_64-pc-windows-msvc build --release --locked
New-Item -ItemType Directory -Path .\desktop-app -Force | Out-Null
Copy-Item -LiteralPath .\target\release\llama-server-configurator.exe -Destination .\desktop-app\llama-server-configurator.exe -Force
```

Check each exit code before continuing; an existing binary may be stale. With cached dependencies, Cargo can use `--offline`. Let Cargo regenerate the lockfile for intentional dependency changes, then return to `--locked`; never edit it manually. Build frontend assets before Rust: plain Cargo does not run Vite automatically.

## Verification

Run from `tauri-app`:

```powershell
node tests/verify-glib-backport.mjs
npm run typecheck
cargo +stable-x86_64-pc-windows-msvc fmt --check
cargo +stable-x86_64-pc-windows-msvc clippy --all-targets --locked -- -D warnings
cargo +stable-x86_64-pc-windows-msvc test --release --locked
npx playwright install chromium --only-shell
npm run test:gui
Get-Item .\desktop-app\llama-server-configurator.exe | Format-List FullName,Length,LastWriteTime
```

Browser checks use mocked services/native event callbacks. Real checks use `tests/native-features.mjs` with `LLAMA_LAUNCHER_EXE`, `LLAMA_SERVER_EXE` and optionally `LLAMA_FIXED_WEBVIEW`; they use isolated data and public fixtures. See [AGENTS.md](AGENTS.md) and [the readiness review](PRODUCTION_READINESS_REVIEW.md) for evidence and remaining gates.

After editing option placement or descriptions, run `node tests/generate-settings-guide.mjs` from `tauri-app`, then `node tests/generate-settings-guide.mjs --check`. The [settings reference](LLAMA_SETTINGS_GUIDE.md) and UI use the same metadata. Syntax/defaults still come from the installed executable, and unknown flags must remain explicitly unreviewed.

Windows validation passed real inference, two-model router accounting, monitoring off, runtime install failure/retry, repeated start/stop and folder relocation. Opening the GUI alone does not prove inference works. No MCP tools are executed by verification.

## Portable package

From `tauri-app`, supply the extracted official x64 Fixed Version WebView2 folder:

```powershell
.\package-portable.ps1 -FixedWebView2Directory 'D:/BuildInputs/WebView2-fixed-x64'
```

The packager includes the EXE, Fixed WebView2, licensed VC runtime DLLs, dependency notices and checksums. Engines, models and private data are excluded. For an already verified binary, `-SkipBuild -ExecutablePath 'target/release/llama-server-configurator.exe'` avoids replacing an open desktop copy. Read [PORTABLE_RELEASES.md](PORTABLE_RELEASES.md) for inputs, signing policy and clean-machine requirements.

## Common failures

| Failure | Action |
|---|---|
| Missing dlltool / import-library error | Select the MSVC toolchain explicitly. |
| EXE access denied during copy/build | Close that copy after coordinating its owned server; distinguish filesystem sandbox restrictions from source errors. |
| Tauri dependency features disagree with allowlist | Preserve Tauri 1 configuration and security policy. Do not restore broad api-all/plugin features blindly. |
| Old frontend / missing index.html | Run npm build from tauri-app; Vite's root is src/frontend and its output is tauri-app/dist. |
| Lockfile needs updating | Regenerate through Cargo only for an intentional dependency change; repeat verification afterward. |
| Save or recovery error | Launch → Saved data & recovery; retain data and protected originals rather than deleting them to make launch succeed. |

The app remains Tauri 1. Manual EXE builds do not require the Tauri CLI; AppImage packaging requires a compatible 1.x CLI. Do not edit protected tauri.conf.json, generated dist/target/node_modules, or deprecated code.
