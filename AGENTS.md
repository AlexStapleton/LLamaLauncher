# LLamaLauncher development

The active application is `tauri-app/`: Rust/Tauri 2 with a Vite frontend. Keep changes in this repository. Do not copy back into the previous project or replace the application with placeholders.

- Rust entry point and configuration validation: `tauri-app/src/main.rs`.
- MCP client/bridge: `tauri-app/src/mcp.rs`; bounded frames: `framing.rs`; private storage: `storage.rs`.
- Versioned DTOs: `config.rs`; server manager: `server.rs`; filesystem scans: `catalog.rs`. Validation remains in main.rs.
- Named launch profile library: `profiles.rs`; typed frontend manager: `src/frontend/profile-ui.ts`. Save full configuration privately; never put profile credentials in browser storage or silently substitute missing models. Validation remains in main.rs.
- Owned router process-tree cleanup: `tauri-app/src/process_tree.rs`.
- Frontend: `tauri-app/src/frontend/`.
- Main application and MCP UI use strict TypeScript (`app.ts`, `mcp-ui.ts`). Keep DOM/IPC contracts current; run the event-driven browser regressions after persistence or connection changes.
- Logs use bounded, credential-scrubbing relays in `logging.rs`; drain both streams even when disk writes fail.
- Damaged private records must remain write-blocked until explicit recovery. Preserve protected backups/originals; async IPC must delegate disk/DPAPI work to workers.
- Managed runtime publication requires a verified install receipt. Preserve unrecognized files and reconcile owned interrupted installs; never infer GPU compatibility from the adapter brand alone.
- The compatible Linux glib backport is in `vendor/`; run `node tests/verify-glib-backport.mjs` and the optimized Linux regression. A Windows pass does not prove Linux readiness.
- Preserve the existing Tauri security configuration; do not manually edit `tauri.conf.json`.
- Build frontend assets before Rust: `npm ci`, `npm run build`.
- Use Node 22.12 or later, Vite 8 and TypeScript. Run `npm run types:generate` when native DTOs change; typecheck rejects stale generated interfaces.
- Keep the protected Tauri configuration and Cargo.lock unchanged. Build.rs rejects stale embedded frontend assets.
- Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, unit/interface checks and a native release build. CI is prepared; distinguish prepared checks from remotely executed checks.
- Windows: use `cargo +stable-x86_64-pc-windows-msvc build --release --locked`. The desktop's default GNU toolchain is missing dlltool. Do not confuse that with source failures.
- Linux: use its native Rust toolchain and install Tauri 2 system libraries. Keep Windows-only APIs behind `#[cfg(windows)]` and Unix APIs behind `#[cfg(unix)]`.
- Do not edit generated `dist`, `target`, or `node_modules`. Let Cargo regenerate the lockfile for intentional dependency changes, then use `--locked`.
- Verify with `npm test`, `cargo test --release --locked`, and the optional Playwright interface harness. Describe headless mocked-backend checks separately from real native/MCP checks.
- Never commit tokens, private MCP profile files, or live connection fixtures. Windows credentials use DPAPI; Linux profiles use account-only file permissions. Do not claim Linux profiles are encrypted.
- MCP tests may initialize and list tools. Do not invoke discovered tools merely to verify a connection.
- Keep GUI rows compact and grouped; descriptions and resource impact belong in expandable help. Light/dark preference persists locally.
- Option placement and reviewed explanations are explicit metadata in `option-guide.js`, not flag-name keyword guesses. Keep GPU weight layers distinct from KV cache. Document direction and conditional magnitude without invented throughput promises; retain executable syntax/defaults and label unknown flags unreviewed. Regenerate `LLAMA_SETTINGS_GUIDE.md` with `node tests/generate-settings-guide.mjs` and verify `--check` after edits.
- Router mode must omit a fixed `--model`, use the selected models directory or cache, and stop owned worker/bridge processes with the router.

Publishing commits or pull requests must follow the user's authorization. The initial remote repository contained only README.md and LICENSE.

## Native feature services

- `monitor.rs`: saved monitoring preferences, cancellable scheduling, bounded history and OS/vendor samples. Monitoring off must stop recurring telemetry, not lifecycle checks. Default two seconds; accepted interval 1–60 seconds.
- `gguf.rs`: bounded metadata parsing/cache and explicit estimate assumptions. Do not load model weights merely to inspect them.
- `session.rs`: owned-server auth/prefix/TCP/Unix socket requests, streaming tests and router events. Test/benchmark requests never execute tools.
- `hub.rs`, `downloads.rs`: protected single-account credentials, public OAuth device flow, immutable revisions, complete shard selection, bounded download concurrency and partial-file ownership. Never put tokens or signed download URLs in job reports.
- `registry.rs`: model/runtime references and official, side-by-side runtime installation. Config schema 3 keeps older raw-path profiles readable. Removal must preserve referenced/imported files.
- `jobs.rs`, `features.rs`: bounded progress/cancellation and IPC adapters. All server startup still passes validation in main.rs.
- Portable packaging scripts preserve the protected Tauri config. Fixed WebView2 and VC redistributables are release inputs; llama.cpp is downloaded during Launch setup or imported from existing files; never package private user data or model weights.
- `tests/native-features.mjs` is opt-in: it downloads a small public model, uses isolated user data, and verifies real CPU inference/router/benchmark behavior. Supply `LLAMA_LAUNCHER_EXE`, `LLAMA_SERVER_EXE` and optionally `LLAMA_FIXED_WEBVIEW`. Browser mock checks do not prove native behavior.

## Portable storage and installer channels

- All managed state is in data beside the executable; AppImage uses the original image's directory, not its temporary mount. LLAMA_LAUNCHER_DATA_DIR is an explicit test override.
- Preserve private file protection and relative references to managed paths. Copy legacy native records without deleting the originals; never copy developer data into packages.
- Stable follows the latest official non-prerelease and its published binary tag when needed. Beta follows prereleases. Never silently substitute Beta for Stable. Hardware/backend selection is automatic; only the release channel is a normal user choice.
- Model folder/picker belongs in Models and stays visible in router mode. Single-model selection disables MCP for the next launch; preserve the connection profile itself.
- Listen address/port belong in Launch. LAN preset binds 0.0.0.0; client previews must use the actual LAN IP or a clearly labeled placeholder, never claim 0.0.0.0 is a client URL. Preserve custom interface addresses and profile settings. Opt-in tests/native-lan.mjs needs LLAMA_LAUNCHER_EXE, LLAMA_SERVER_EXE and LLAMA_MODEL_FILE; it briefly opens an ephemeral LAN listener without changing firewall rules.

## Public packaging (1.1.1+)

- Publish Windows NSIS setup EXE and Linux AppImage, not raw executable ZIPs. Preserve previous release tags/assets.
- `package-windows.ps1` stages known files and compiles `installer/windows.nsi`. Never package data or fixtures. Uninstall deletes only exact packaged paths and preserves data/unrecognized files. Installation is per user so adjacent data stays writable. Installer smoke runs only on an isolated CI runner.
- `package-appimage.sh` invokes the locked @tauri-apps/cli 2.x using an ignored configuration override; do not change tauri.conf.json. `release-linux.sh` verifies bundled WebKitGTK/helpers and requires Docker clean-host startup without WebKitGTK installed.
- The native 1.1 Windows/Linux workflow completed successfully; older review statements about unexecuted Linux are historical. Do not claim broader AppImage/hardware/clean-machine checks passed without their own evidence.

- The user explicitly authorized the Tauri 2 migration for 1.1.1. The configuration is now schema 2; preserve its CSP and local main-window event capabilities. Do not reapply old Tauri 1 instructions. Linux uses WebKitGTK 4.1/libsoup 3 on Ubuntu 24.04. `tests/linux-package-inventory.mjs` gates the engine version and records the actual native package versions.
