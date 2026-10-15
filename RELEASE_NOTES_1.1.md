# LLamaLauncher 1.1

LLamaLauncher 1.1 adds a complete llama.cpp setup, model library and launch-profile workflow, with clearer settings and a more compact desktop interface.

- Download official llama.cpp Stable/Beta builds from Launch, with automatic backend validation and CPU fallback. Existing executables remain supported.
- Search Hugging Face, browse GGUF files/shards, download verified models, and connect a read-token account for gated/private access.
- Keep named profiles, working settings, engines, default model downloads and logs in `data` beside the launcher. Windows private records use account-bound DPAPI; Linux uses account-only permissions.
- Organize settings by purpose, including dedicated Context & cache and Chat & output tabs. All 257 bundled option groups have reviewed explanations, direction of change, conditional magnitude examples and separate speed/VRAM/RAM guidance. Installed executable syntax/defaults remain authoritative.
- Manage single-model and router launches, MCP connections/tool discovery, model loading and owned server-process cleanup.
- Use adjustable hardware monitoring, a complete polling-off switch, GGUF inspection, launch preflight, inference tests and saved benchmark comparisons.
- Configure LAN listen addresses and ports directly in Launch. Preserve custom interface bindings in profiles.
- Recover damaged saved records explicitly, retry failed saves, retain protected backups and rotate credential-scrubbed server logs within a bounded disk budget.
- Avoid hidden settings controls, redundant catalog redraws and search callbacks that finish after leaving a tab. Unchanged runtime help preserves active controls; changed help still updates the option list.

## Downloads and setup

**Windows x86_64:** extract the entire Windows ZIP and run `LLamaLauncher/llama-server-configurator.exe`. Keep the included `WebView2` folder and runtime DLLs beside the EXE. The Windows package contains a private fixed WebView2 runtime; no separate WebView2 setup is required.

**Linux x86_64:** extract the Linux ZIP and run `LLamaLauncher/llama-server-configurator`. The native Linux archive requires GTK 3, WebKitGTK 4.0 and AppIndicator libraries, as on Ubuntu 22.04. It is not an AppImage and does not bundle those system libraries.

Model weights and llama.cpp are supplied separately through Launch and Models. No user data or developer credentials are included in either archive. Each archive has a SHA256 checksum; an offline `LLAMA_SETTINGS_GUIDE.md` and dependency notices are included.

When upgrading, keep an account-protected backup of the existing `data` directory and preserve it while replacing application/runtime files. Windows credentials remain tied to the same Windows account. External model/executable paths may need relinking after moving computers.

## Validation and status

The release pipeline must pass Windows/Linux frontend builds, type/interface checks, Rust formatting/lints/release tests, browser regressions and a native Linux GUI window check before publishing. Windows packaging also tests fresh startup with its bundled WebView2. MCP checks list tools without executing them.

This is an **unsigned beta release**. Clean-machine deployment, the wider GPU/driver matrix, registered browser OAuth and extended stress/soak testing remain acceptance gates. Native CI and local checks do not establish that those broader scenarios passed. General startup/search benchmark timings were essentially unchanged by the focused optimization pass; its demonstrated benefit is removing redundant work and edit disruption.

See [README](https://github.com/AlexStapleton/LLamaLauncher/blob/1.1/README.md), [settings guide](https://github.com/AlexStapleton/LLamaLauncher/blob/1.1/LLAMA_SETTINGS_GUIDE.md), [changelog](https://github.com/AlexStapleton/LLamaLauncher/blob/1.1/CHANGELOG.md) and [readiness review](https://github.com/AlexStapleton/LLamaLauncher/blob/1.1/PRODUCTION_READINESS_REVIEW.md).
