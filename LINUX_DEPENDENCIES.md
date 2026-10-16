# Linux dependency review — 2026-10-16

## Migration implementation

The user selected modernization before publication. Source is migrated to stable Tauri 2.12.4, Wry 0.57.0, Rust WebKitGTK binding 2.0.2 and soup3 0.5.0, JavaScript API 2.12.4 and CLI 2.12.3. These are Rust/npm package versions, not native engine versions. The Linux build baseline is Ubuntu 24.04 (glibc 2.39), using apt updates/security packages. Publication requires WebKitGTK engine 2.54 or newer, records exact native package versions and rejects bundled WebKitGTK 4.0/libsoup 2. Tauri 2 still uses GTK 3 and GLib Rust binding 0.18.5; the iterator fix is carried forward in a fresh verified 0.18.5 source backport, replacing the 0.15 fork. The yanked zip 2.4.2 archive dependency is replaced by current zip 9.0.4; archive containment/link/size tests remain required. Native Windows/Linux and clean-host package validation must pass before publication.

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
