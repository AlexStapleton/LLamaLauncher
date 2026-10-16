# Dependency review — 2026-10-16

This is an unreleased source update after 1.1.2. Published release tags/assets are unchanged. The policy is latest compatible stable, with explicit exceptions. A successful vulnerability audit is not evidence that every package is current or that native libraries have no vulnerabilities.

The user chose Debian 13/newer as the Linux target. New source packages use glibc 2.41 or newer; Ubuntu 24.04 is no longer a binary compatibility target. CI uses an isolated Debian 13 build container, a separate cache identity and separate clean Debian 13/Ubuntu 26.04 startup checks. The root-only WebKit sandbox override applies to test processes, not the production launcher.

The first clean Debian check of the earlier Ubuntu-built image detected omitted libgpg-error and libcom_err, despite Ubuntu startup passing. Both support libraries and their native copyright notices are now explicitly bundled. Clean-host `ldd` checks cover the launcher and WebKit helpers before GUI startup; installing those libraries on the clean host would hide the packaging defect.

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
| NSIS compiler | 3.11 | 3.13 (Chocolatey 3.13.0) | Latest upstream stable; warnings remain fatal. Windows CI passed installed GUI, registration, upgrade preservation and registered uninstall checks. |
| Fixed WebView2 x64 | 155.0.4283.45 | 155.0.4283.62 | Latest vendor Fixed x64 download at review cutoff; immutable URL/hash in `runtime-inputs.json`, Microsoft signature/version checked. |
| App-local VC runtime | Inherited from installed Visual Studio; local example 14.44.35211.0 | 14.51.36247.0 | Latest signed Microsoft redistributable. Extract only verified x64 DLLs from its hash-pinned cabinets; do not install a system runtime. Record every DLL's version/hash/signer in `WINDOWS_PACKAGE_VERSIONS.json`. |
| GitHub Actions | Floating major tags | Latest stable release commits | checkout 7.0.1, setup-node 7.1.0, upload-artifact 7.0.2, rust-cache 2.9.2; immutable commit pins with version comments. |

Tauri 2.12.4, its JavaScript API 2.12.4, CLI 2.12.3, Vite 8.3.4, TypeScript 7.0.2, zip 9.0.4 and the remaining direct Rust application libraries were already on the latest stable versions found by the registry review. The Linux-only GLib binding is the direct exception described below. Rust stable is 1.99.0 in this validation; the manifest now requires the tested 1.99 toolchain rather than implying that the entire new graph was validated on the old 1.89 minimum.

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
| Linux native WebKitGTK | Moved from Ubuntu's 2.52.6 engine to the Debian 13 security/updates baseline >=2.54.0. Upstream 2.54.1 is newer than Debian's currently published 2.54.0-1~deb13u2. | User selected the newer distro baseline. Keep this remaining upstream patch gap explicit and review actual Debian patch coverage. Do not mix binaries from Debian unstable or claim all upstream fixes are present. |
| Native GLib, GTK, libsoup, OpenSSL and other system libraries | New source builds use Debian 13 security/updates packages, separately from Rust bindings. OpenSSL minimum is now 3.5.7 on the supported 3.5 LTS ABI, replacing Ubuntu's upstream-EOL 3.0 line. Debian publishes 3.5.7-1~deb13u3; upstream 3.5.9 LTS and 4.0.3 are newer. | Record exact binary/source package versions for the entire apt build host. Preserve supported vendor ABI/patched packages rather than silently installing an incompatible major. Native patch coverage remains a separate review from Rust/npm advisories. |
| AppImage deployment helpers | Tauri CLI controls its Linux deployment-helper downloads, separately from the application Cargo lockfile. | Track through the pinned CLI and packaged hashes; the application lockfile alone does not pin all external helper bytes. This remains a reproducibility limitation. |

The [WebKit security advisory WSA-2026-0006](https://webkitgtk.org/security/WSA-2026-0006.html) must be assessed against actual distro patches and enabled components; the version gate alone cannot establish patch coverage. [OpenSSL's upstream support/download policy](https://openssl-library.org/source/) is distinct from Debian's patched distro release. Remaining upstream-versus-vendor patch gaps are not hidden by zero Rust/npm advisory results. Sources for the new distro baselines: [Debian WebKitGTK](https://packages.debian.org/trixie/libwebkit2gtk-4.1-0), [Debian OpenSSL](https://packages.debian.org/trixie/libssl3t64).

The executed Debian 13 build inventory reports WebKitGTK 2.54.0, native GLib 2.84.4, GTK 3.24.49, libsoup 3.6.5 and OpenSSL 3.5.7. Full distro patch suffixes and all apt binary/source package versions are recorded separately in the package inventory. Official upstream inventories currently list GLib 2.90.1, GTK 3.24.52 within the GTK 3 series, libsoup 3.8.0, and OpenSSL 4.0.3 (3.5.9 is the current LTS line). GTK 4 is a separate API generation. Sources: [GLib](https://download.gnome.org/sources/glib/cache.json), [GTK 3](https://download.gnome.org/sources/gtk/3.24/), [libsoup](https://download.gnome.org/sources/libsoup/cache.json), [OpenSSL](https://openssl-library.org/source/). The old `gtk+` archive stops at older releases; it must not be used to infer the current GTK 3 version.

Debian source archives were checked against their published `.dsc` SHA256 entries before reading patch notes. OpenSSL 3.5.7-1~deb13u3 contains September 29 security backports covering DTLS, CRLDP, QUIC, EC/SM2 and CMP issues; it is not unmodified upstream 3.5.7. WebKitGTK 2.54.0-1~deb13u2 includes fixes for Eclipse startup and web-process aborts, dated October 2. This establishes those particular backports, not full equivalence with every newer upstream change. Sources: [OpenSSL source descriptor](https://security.debian.org/debian-security/pool/updates/main/o/openssl/openssl_3.5.7-1~deb13u3.dsc), [WebKitGTK source descriptor](https://security.debian.org/debian-security/pool/updates/main/w/webkit2gtk/webkit2gtk_2.54.0-1~deb13u2.dsc).

Tauri's external packaging inputs were also reviewed: [linuxdeploy-07333c6](https://github.com/tauri-apps/binary-releases/releases/tag/linuxdeploy-07333c6) was published September 20, 2026; its x64 asset digest is `36a2d7e274d12e1050d0e9ecfe11d339ed54720b2bec464c286d53f8b07f5c62`. The small [AppRun compatibility launcher](https://github.com/tauri-apps/binary-releases/releases/tag/apprun-old) dates to July 20, 2025, digest `f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f`. The [rolling AppImage output plugin](https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/tag/continuous) x64 asset was updated October 1, 2026, digest `49d6a17160675a6bd1781699aae6bdf7692d98552e02a3671d2183d10547842e`. These are observed upstream asset digests, not a claim of separately enforced helper locks. Keep helper servicing/reproducibility distinct from the application's Cargo lockfile.

## Security, validation and follow-up

cargo-audit 0.22.2 against RustSec database commit `479c888ce4401313bca61ba4aba2d0f6244c4a09` reports zero known vulnerabilities, zero yanked locked packages, and the one unmaintained proc-macro-error warning. The path-patched GLib dependency is verified separately; it is not covered by the registry advisory result. npm audit reports zero vulnerabilities across the lockfile.

Executed locally: 15 frontend tests, strict type checking and production build on Node 26; browser UI regression and measurements; Rust formatting/Clippy; 61 optimized backend tests (three fixture entrypoints ignored, with parent tests exercising them); native Windows model selection/download/verification, inference, router accounting/load/unload, benchmark, hardware detection, monitoring off, repeated start/stop, runtime install/remove/receipt recovery and portable settings. Tests used synthetic private state/public models and executed zero MCP tools. A fresh staged GUI also passed with the new VC DLLs and Fixed WebView2.

[CI run 38957016803](https://github.com/AlexStapleton/LLamaLauncher/actions/runs/38957016803) completed successfully for source commit `2a764c3701638b1f8fd7c3bb23e09b47c050032b`. Both platforms passed frontend, strict type, formatting, Clippy, browser regression and native release checks. Windows passed 61 optimized backend tests; Debian passed 66, including its Linux-only coverage (each reports three ignored fixture entrypoints). The NSIS 3.13 installer passed real installed-GUI startup, Installed apps registration, upgrade preservation and the registered uninstaller test. The Debian-built AppImage passed separate clean Debian 13 and Ubuntu 26.04 startup checks without system WebKitGTK or FUSE. The executed Linux inventory confirms WebKitGTK `2.54.0-1~deb13u2`, OpenSSL `3.5.7-1~deb13u3` and glibc `2.41-12+deb13u4`. Minimal-container packaging prerequisites and omitted support libraries found during earlier failed runs were corrected before this successful run.

The separate [dependency-review CI run 38954940485](https://github.com/AlexStapleton/LLamaLauncher/actions/runs/38954940485) also passed. Build artifacts and evidence are attached to the successful runs; they are unreleased test packages, even though their filenames retain the source version 1.1.2. Existing published 1.1.2 assets retain their original source and hashes. These checks do not establish coverage of every GPU/driver, desktop integration or extended inference session.

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
