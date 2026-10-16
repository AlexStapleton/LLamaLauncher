# Dependency review — 2026-10-16

This is an unreleased source update after 1.1.2. Published release tags/assets are unchanged. The policy is latest compatible stable, with explicit exceptions. A successful vulnerability audit is not evidence that every package is current or that native libraries have no vulnerabilities.

## Scope and reproducible evidence

[DEPENDENCY_INVENTORY.json](DEPENDENCY_INVENTORY.json) records every resolved Rust package across target platforms and every npm lockfile package, including optional platform binaries. [The readable inventory](DEPENDENCY_INVENTORY.md) lists every older instance with its parent requirements. It covers 474 Rust instances and 76 npm packages. After updates, 133 Rust instances and one npm package remain behind the registry's latest stable release; different versions of the same package count separately. This is wider than the dependencies actually shipped on Windows or Linux. Absent optional npm peers are not installed merely to make `npm outdated --all` quiet.

The inventory tool reads official Cargo sparse-index and npm registry data. `cargo update --dry-run --verbose` confirms no ordinary compatible updates remain. cargo-outdated 0.19.0 was also run, but its resolver-oriented output misses many upstream major-version gaps; it is not the inventory's source of truth.

## Changes

| Component | Previous | Selected | Reason / evidence |
| --- | --- | --- | --- |
| Application sysinfo | 0.37.2 | 0.39.6 | Latest stable; monitor/disk APIs compile and native hardware/monitoring checks pass. |
| Application sha2 | 0.10.9 | 0.11.1 | Latest stable; output formatting adapted without changing hashes or saved IDs. Known SHA-256 vectors and real model/runtime verification pass. |
| Playwright / core | 1.64.0 | 1.64.1 | Latest stable patch; full UI regression passes. |
| Indirect Rust dependencies | Previous Cargo lock | 10 compatible updates plus new direct-library dependencies | Includes schannel, plist, quick-xml, combine, equivalent and defmt family. Generated with Cargo. |
| Indirect npm dependencies | Previous npm lock | Refreshed compatible graph | Updated Playwright and current oxc/rolldown platform bindings. Generated with npm. |
| Node / npm build tools | 22.23.3 / 10.9.9 | 26.12.0 / 11.20.0 | Latest stable Current build toolchain; `.nvmrc` records it. Node is not shipped or used as the desktop application's server. Current LTS 24.22.0 remains an alternative, not the selected CI version. |
| NSIS compiler | 3.11 | 3.13 (Chocolatey 3.13.0) | Latest upstream stable; warnings remain fatal. Windows CI must exercise the resulting installer. |
| Fixed WebView2 x64 | 155.0.4283.45 | 155.0.4283.62 | Latest vendor Fixed x64 download at review cutoff; immutable URL/hash in `runtime-inputs.json`, Microsoft signature/version checked. |
| App-local VC runtime | Inherited from installed Visual Studio; local example 14.44.35211.0 | 14.51.36247.0 | Latest signed Microsoft redistributable. Extract only verified x64 DLLs from its hash-pinned cabinets; do not install a system runtime. Record every DLL's version/hash/signer in `WINDOWS_PACKAGE_VERSIONS.json`. |
| GitHub Actions | Floating major tags | Latest stable release commits | checkout 7.0.1, setup-node 7.1.0, upload-artifact 7.0.2, rust-cache 2.9.2; immutable commit pins with version comments. |

Tauri 2.12.4, its JavaScript API 2.12.4, CLI 2.12.3, Vite 8.3.4, TypeScript 7.0.2, zip 9.0.4 and the remaining direct Rust application libraries were already on the latest stable versions found by the registry review. The Linux-only GLib binding is the direct exception described below. Rust stable is 1.99.0 in this validation; `rust-version = 1.89` expresses a source minimum, not the compiler used by CI or proof that the entire graph is tested on that minimum.

## Retained constraints and servicing gaps

| Dependency family | Concrete reason for older versions | Decision |
| --- | --- | --- |
| GTK/GLib/GIO/Cairo/Pango Rust bindings | Tauri/Wry's GTK 3 graph requires gtk 0.18 and glib 0.18. Newest GLib Rust binding is 0.22.10; substituting it does not satisfy the existing Rust type/API graph. | Retain the compatible graph. Verify the GLib 0.18.5 out-pointer security backport and optimized Linux regression. Do not add incompatible global overrides. |
| proc-macro-error 1.0.4 | GTK's macro chain still uses this unmaintained build dependency (RUSTSEC-2024-0370). No newer release of the same package removes that status. | Keep visible as an unresolved maintenance exception; replacement requires upstream macro-chain migration. It is not a clean bill of maintenance. |
| Tauri's SHA-256 0.10 graph | tauri-codegen and Wry require sha2 `^0.10`; crypto-common 0.1.7 pins generic-array `=0.14.7`. | Application code now uses sha2 0.11.1. Retain the framework's separate older build/runtime instances until upstream migrates. |
| Brotli decompressor 6.0.1 | brotli 9.0.0 requires `~6.0`, excluding 6.1.0. | Retain declared upstream compatibility; no forced minor override. |
| TOML 0.8 / old procedural macro helpers | proc-macro-crate 2.0.2 pins toml_edit `=0.20.2` and toml_datetime `=0.6.3`; these prevent the apparently compatible TOML patch updates. | Record exact constraints; migrate through owning GTK macros rather than overriding types independently. |
| Other old Rust lines, including Windows SDK, parsers, schema generators and platform-only packages | Every instance and its immediate parent requirements are listed in the inventory. Many packages have multiple simultaneously required major versions. | Retain parent-supported APIs, check advisories, and review upstream parent upgrades. Newest available is not synonymous with a drop-in replacement. |
| nanoid 3.3.20 versus 6.0.2 | Latest PostCSS still declares its compatible 3.x dependency and consumes its CommonJS API. This is a build dependency. | Retain PostCSS's supported range; npm audit reports no advisory. Do not force a new major through `overrides`. |
| Linux native WebKitGTK 2.52.6 versus upstream 2.54.1 | Existing AppImage baseline is Ubuntu 24.04/glibc 2.39. Its security/updates archive supplies this engine. Debian 13 currently supplies 2.54.0 and uses a newer glibc. | Explicit remaining freshness/servicing gap. Changing the native build distribution can raise the minimum OS requirement. Building newer WebKit on the old ABI is separate native-runtime work; do not claim it is impossible or already complete. Add clean Debian 13 startup validation now; keep the existing baseline pending the user's compatibility choice. |
| Native GLib, GTK, libsoup, OpenSSL and other system libraries | Native versions come from the selected distro, separately from similarly named Rust crates. Ubuntu OpenSSL 3.0 is past upstream community support; Ubuntu's vendor security patching is a separate servicing policy. | Record exact binary/source package versions for the entire apt build host. No claim that distro packages equal latest upstream or have zero native vulnerabilities. This gap must be revisited with the Linux build baseline. |
| AppImage deployment helpers | Tauri CLI controls its Linux deployment-helper downloads, separately from the application Cargo lockfile. | Track through the pinned CLI and packaged hashes; the application lockfile alone does not pin all external helper bytes. This remains a reproducibility limitation. |

The [WebKit security advisory WSA-2026-0006](https://webkitgtk.org/security/WSA-2026-0006.html) must be assessed against actual distro patches and enabled components; the version gate alone cannot establish patch coverage. [OpenSSL's upstream support/download policy](https://openssl-library.org/source/) is distinct from Ubuntu's patched distro release. These are open native servicing issues, not hidden by the zero Rust/npm advisory results.

The last executed Ubuntu package inventory used native GLib 2.80.0, GTK 3.24.41, libsoup 3.4.4 and OpenSSL 3.0.13, with full distro patch suffixes recorded separately. Official upstream release inventories currently list GLib 2.90.1, GTK 3.24.43 within the GTK 3 series, libsoup 3.8.0, and OpenSSL 4.0.3 (3.5.9 is the current LTS line). GTK 4 is a separate API generation. Sources: [GLib](https://download.gnome.org/sources/glib/cache.json), [GTK 3](https://download.gnome.org/sources/gtk+/cache.json), [libsoup](https://download.gnome.org/sources/libsoup/cache.json), [OpenSSL](https://openssl-library.org/source/). These comparisons expose remaining native version gaps; they do not approve mixing incompatible distro binaries or replacing them without native tests.

## Security, validation and follow-up

cargo-audit 0.22.2 against RustSec database commit `479c888ce4401313bca61ba4aba2d0f6244c4a09` reports zero known vulnerabilities, zero yanked locked packages, and the one unmaintained proc-macro-error warning. The path-patched GLib dependency is verified separately; it is not covered by the registry advisory result. npm audit reports zero vulnerabilities across the lockfile.

Executed locally: 15 frontend tests, strict type checking and production build on Node 26; browser UI regression and measurements; Rust formatting/Clippy; 61 optimized backend tests (three fixture entrypoints ignored, with parent tests exercising them); native Windows model selection/download/verification, inference, router accounting/load/unload, benchmark, hardware detection, monitoring off, repeated start/stop, runtime install/remove/receipt recovery and portable settings. Tests used synthetic private state/public models and executed zero MCP tools. A fresh staged GUI also passed with the new VC DLLs and Fixed WebView2. Native Linux builds, clean Ubuntu/Debian containers and the NSIS 3.13 installed-GUI/upgrade/uninstall tests must be verified in CI for this commit; prepared checks are not execution evidence.

Weekly Dependabot updates cover Cargo, npm and GitHub Actions. A read-only dependency-review workflow publishes complete registry inventories, external-input freshness and Rust/npm advisory reports. It also runs when its scripts/runtime-input definition change. The VC check reads version metadata and hashes from the latest official bundle without executing it; pin adoption still requires Windows publisher/architecture validation. The workflow does not install updates, invoke MCP tools or publish releases. Main-branch CI now also packages and tests ordinary dependency changes, so packaging cannot silently regress between releases. External runtime pins still require explicit review and checksum/signature verification.

Re-run from `tauri-app/`:

```sh
node tests/dependency-inventory.mjs --output ../DEPENDENCY_INVENTORY.json
node tests/upstream-inputs.mjs
npm audit
cargo audit
cargo update --dry-run --verbose
```

On Windows, select `+stable-x86_64-pc-windows-msvc` for Cargo commands. Official sources: [Cargo index](https://index.crates.io/), [npm registry](https://registry.npmjs.org/), [Node releases](https://nodejs.org/en/about/previous-releases), [NSIS release data](https://nsis-dev.github.io/release-data/versions.json), [Microsoft Fixed WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/), [Microsoft VC runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist), [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), and [WebKitGTK](https://webkitgtk.org/).
