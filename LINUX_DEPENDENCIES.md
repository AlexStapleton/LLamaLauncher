# Linux dependency review — 2026-10-16

## Current unreleased baseline: Debian 13

The user selected Debian 13 and newer distributions during the full dependency pass. Source builds now run inside Debian 13 with glibc 2.41, WebKitGTK >=2.54.0 and OpenSSL >=3.5.7. Ubuntu 24.04 remains only the GitHub Docker host. Clean startup targets Debian 13 and Ubuntu 26.04 without installed WebKitGTK/FUSE; the compatibility job is separate from the build container and is required before publication. This does not replace existing 1.1.2 assets or change their glibc 2.39 baseline. Current source/package evidence and remaining upstream-vs-distro gaps are in [DEPENDENCY_REVIEW.md](DEPENDENCY_REVIEW.md).

## Migration implementation (published 1.1.2 history)

The user selected modernization before publication. Source is migrated to stable Tauri 2.12.4, Wry 0.57.0, Rust WebKitGTK binding 2.0.2 and soup3 0.5.0, JavaScript API 2.12.4 and CLI 2.12.3. These are Rust/npm package versions, not native engine versions. The Linux build baseline is Ubuntu 24.04 (glibc 2.39), using apt updates/security packages. Publication requires WebKitGTK engine 2.52.6 or newer, records exact native package versions and rejects bundled WebKitGTK 4.0/libsoup 2. Tauri 2 still uses GTK 3 and GLib Rust binding 0.18.5; the iterator fix is carried forward in a fresh verified 0.18.5 source backport, replacing the 0.15 fork. The yanked zip 2.4.2 archive dependency is replaced by current zip 9.0.4; archive containment/link/size tests remain required. Native Windows/Linux and clean-host package validation must pass before publication.

## Distro baseline versus upstream

Ubuntu Launchpad currently publishes `2.52.6-0ubuntu0.24.04.1` for WebKitGTK 4.1 in Noble Security and Updates (published 2026-08-31). Upstream lists 2.54.1 as newest stable. These are different release policies: the package gate accepts the current supported Ubuntu security baseline, records the exact packaged version, and does not claim it is newest upstream. Rebuild with Ubuntu security updates when servicing the bundled runtime. Upstream advisories and Ubuntu patch coverage must be reviewed during servicing; an apt security-pocket label or a zero RustSec result does not prove the native WebKit engine has no vulnerabilities. The broad production sign-off remains open.

Package inventory source: [Ubuntu Launchpad primary archive API](https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedBinaries&binary_name=libwebkit2gtk-4.1-0&exact_match=true&distro_arch_series=https://api.launchpad.net/1.0/ubuntu/noble/amd64&status=Published).

## Original finding

The current Tauri 1 runtime is an older compatibility stack, not a modernized Linux platform. AppImage bundling fixes missing host libraries but does not change this fact. Do not present a successful AppImage build as a dependency modernization.

| Component | Resolved/current project use | Assessment |
| --- | --- | --- |
| Tauri | Rust 1.8.3; CLI 1.6.3 | Previous framework generation. Move to Tauri 2 to use the supported newer Linux WebKit binding. |
| Wry | 0.24.12 | Bound to the Tauri 1 generation. Upgrade through Tauri, not by overriding its transitive crate version. |
| WebKitGTK | native `webkit2gtk-4.0` API; Rust binding 0.18.2 | Deprecated libsoup 2 API. This is the main platform modernization problem. The API suffix is not the engine release version. Record the actual Ubuntu package/engine version in each build. |
| libsoup | native 2.4 API; Rust binding soup2 0.2.1 | Legacy network stack; upstream WebKitGTK 2.52 removed libsoup 2 support. Migrate to libsoup 3 via WebKitGTK 4.1/Tauri 2. |
| GTK | native GTK 3; Rust gtk 0.15.5 | GTK 3 is old stable, distinct from unsupported/dead software. Tauri 2 also uses GTK 3, so a Tauri migration does not imply GTK 4. Do not force incompatible GTK 4 crates into Tauri. |
| GLib binding | local 0.15.12 source backport | A documented upstream out-pointer fix is backported and its optimized native regression passes in 1.1 Linux CI. Modernization should remove this fork after the newer graph is verified. The Rust binding version is not the native GLib package version. |
| HTTP client/TLS | reqwest 0.13.5; openssl Rust binding 0.10.81 | Separate from WebKitGTK/libsoup. Native OpenSSL versions must be recorded from the build environment. |
| Build baseline | Ubuntu 22.04 with apt updates/security repositories | Chosen for glibc compatibility, not because every system library is current upstream. An old ABI baseline alone is not a security status claim. |

## Required modernization

1. Migrate Rust Tauri/build crates, JavaScript APIs, dialogs, events, window APIs and permissions together to stable Tauri 2. Preserve the CSP restrictions and private storage semantics.
2. Replace WebKitGTK 4.0/libsoup 2 with WebKitGTK 4.1/libsoup 3; keep GTK 3 while required by Tauri. Refresh the Cargo/npm lockfiles through their package managers.
3. Remove the old GLib source backport only after the new dependency graph uses a fixed upstream release; repeat native regressions and dependency/security checks.
4. Rebuild AppImage using current security-updated packages. Capture native package/engine versions in an artifact and include them in the image's documentation.
5. Test on clean Ubuntu 24.04 and another current distribution without a host WebKitGTK installation, plus Windows native/installer regressions. Verify the image actually contains the intended WebKit runtime and helpers.

The framework/configuration migration is a separate functional change from the 1.1.1 packaging recipe. It needs native validation on both platforms before publication; merely changing an apt package name cannot make the old bindings link against a different ABI.

## Primary sources

- [WebKitGTK libsoup 2 deprecation/removal](https://webkitgtk.org/2025/10/07/webkitgtk-soup2-deprecation.html): WebKitGTK 2.52 removed support; 4.0 uses libsoup 2 and 4.1 uses libsoup 3.
- [Current WebKitGTK releases](https://webkitgtk.org/): stable engine 2.54.1 listed on the review date. This is different from an API name such as 4.1.
- [Tauri migration guide](https://v2.tauri.app/start/migrate/from-tauri-1/): configuration, IPC, plugins and permissions must migrate together.
- [Tauri 2 Linux prerequisites](https://v2.tauri.app/start/prerequisites/): WebKitGTK 4.1.
- [GTK upstream](https://www.gtk.org/): GTK 4 is current stable; GTK 3 is labeled old stable.
- [Tauri AppImage guidance](https://v2.tauri.app/distribute/appimage/): use the oldest supported baseline with the required WebKitGTK ABI, and test actual distributions.

Dependency audit after migration: cargo-audit 0.22.2 against RustSec commit `479c888ce4401313bca61ba4aba2d0f6244c4a09` (2026-10-15) reports zero vulnerabilities and one unmaintained build-macro warning (`proc-macro-error` 1.0.4). The local GLib source fix is verified separately and is not covered by the path-dependency audit. npm install reports zero vulnerabilities. This is not a native Linux C-library vulnerability scan.

Tauri 2 CSP retains the existing source restrictions and adds only its documented native IPC transports (`ipc:` and `http://ipc.localhost`) to connect-src. The GUI still has no arbitrary remote network/script permission.

The first clean Ubuntu container check detected omitted libfribidi even though build-host AppImage startup passed. AppImage configuration now explicitly includes the system text-shaping library set and corresponding licenses. Clean-host checks also run ldd on the launcher and all WebKit helper executables before launching the GUI. The Windows window retains useHttpsScheme=true to preserve Tauri 1 browser storage origins.

## Executed Linux package validation

GitHub Actions [run 38944126774](https://github.com/AlexStapleton/LLamaLauncher/actions/runs/38944126774), Linux job 117118822627, passed all source/native/browser checks and AppImage packaging from commit `ba665dd91d9fdcd536b7b1cd2f0d2fe43a2804ec`. The clean Ubuntu 24.04 container had no installed WebKitGTK and used FUSE-free extraction; dependency checks for the launcher and WebKit helpers passed, the real window initialized, and frontend settings were saved beside the original AppImage. The bundled text-library fix was exercised. This is Linux startup evidence, not a GPU/driver or long-running inference certification.

Release 1.1.1 passed Windows/Linux CI and was published from `9d3d2871cec51c80cb3da323a75679db531c8686`. Final compiler-log review found a Windows uninstall registration quoting warning that direct uninstaller execution did not catch. The 1.1.2 patch fixes and checks both registry command strings, exercises the registered path, and makes NSIS warnings fatal. Linux dependency choices are unchanged; packages are rebuilt for 1.1.2.

## Subsequent dependency pass

See [DEPENDENCY_REVIEW.md](DEPENDENCY_REVIEW.md) for the full Rust/npm/native/build-tool inventory. The selected build baseline is now Debian 13/glibc 2.41, with its security/updates repositories; this is newer than the published 1.1.2 baseline but still distinct from newest upstream releases. Refer to actual CI execution evidence before claiming the new container/build checks passed.
