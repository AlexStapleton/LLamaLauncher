# LLamaLauncher development

The active application is `tauri-app/`: Rust/Tauri 1 with a Vite frontend. Keep changes in this repository. Do not copy back into the previous project or replace the application with placeholders.

- Rust entry point and configuration validation: `tauri-app/src/main.rs`.
- MCP client/bridge: `tauri-app/src/mcp.rs`; bounded frames: `framing.rs`; private storage: `storage.rs`.
- Versioned DTOs: `config.rs`; server manager: `server.rs`; filesystem scans: `catalog.rs`. Validation remains in main.rs.
- Named launch profile library: `profiles.rs`; typed frontend manager: `src/frontend/profile-ui.ts`. Save full configuration privately; never put profile credentials in browser storage or silently substitute missing models. Validation remains in main.rs.
- Owned router process-tree cleanup: `tauri-app/src/process_tree.rs`.
- Frontend: `tauri-app/src/frontend/`.
- Preserve the existing Tauri security configuration; do not manually edit `tauri.conf.json`.
- Build frontend assets before Rust: `npm ci`, `npm run build`.
- Use Node 22.12 or later, Vite 8 and TypeScript. Run `npm run types:generate` when native DTOs change; typecheck rejects stale generated interfaces.
- Keep the protected Tauri configuration and Cargo.lock unchanged. Build.rs rejects stale embedded frontend assets.
- Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, unit/interface checks and a native release build. CI is prepared; distinguish prepared checks from remotely executed checks.
- Windows: use `cargo +stable-x86_64-pc-windows-msvc build --release --locked`. The desktop's default GNU toolchain is missing dlltool. Do not confuse that with source failures.
- Linux: use its native Rust toolchain and install Tauri 1 system libraries. Keep Windows-only APIs behind `#[cfg(windows)]` and Unix APIs behind `#[cfg(unix)]`.
- Do not edit generated `dist`, `target`, or `node_modules`. Let Cargo regenerate the lockfile for intentional dependency changes, then use `--locked`.
- Verify with `npm test`, `cargo test --release --locked`, and the optional Playwright interface harness. Describe headless mocked-backend checks separately from real native/MCP checks.
- Never commit tokens, private MCP profile files, or live connection fixtures. Windows credentials use DPAPI; Linux profiles use account-only file permissions. Do not claim Linux profiles are encrypted.
- MCP tests may initialize and list tools. Do not invoke discovered tools merely to verify a connection.
- Keep GUI rows compact and grouped; descriptions and resource impact belong in expandable help. Light/dark preference persists locally.
- Router mode must omit a fixed `--model`, use the selected models directory or cache, and stop owned worker/bridge processes with the router.

Publishing commits or pull requests must follow the user's authorization. The initial remote repository contained only README.md and LICENSE.
