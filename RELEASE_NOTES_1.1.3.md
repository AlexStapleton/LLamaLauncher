# LLamaLauncher 1.1.3

One portable Windows executable and one Linux AppImage, with improved readability, Linux XDG storage, and refreshed dependencies.

## Downloads

- **Windows x86_64:** `LLamaLauncher-1.1.3-Windows-x86_64.exe`. Place it in a writable folder and double-click. The GUI runtime is embedded; there is no setup wizard. Settings, profiles, downloaded llama.cpp engines and default model downloads stay in `data` beside the original EXE. Keep that folder when replacing the executable. Credentials remain tied to the same Windows account.
- **Linux x86_64:** `LLamaLauncher-1.1.3-Linux-x86_64.AppImage`. Targets Debian 13 and newer distributions with glibc 2.41 or newer. WebKitGTK 4.1/libsoup 3 and required helpers are bundled. Debian 12, Ubuntu 24.04, ARM and Alpine/musl are outside this package's target.

```sh
chmod +x LLamaLauncher-1.1.3-Linux-x86_64.AppImage
./LLamaLauncher-1.1.3-Linux-x86_64.AppImage
```

Without FUSE, add `--appimage-extract-and-run`. SHA-256 files and `SHA256SUMS.txt` accompany the downloads. Model weights and llama.cpp are downloaded separately inside the application or imported from existing files.

## Changes

- Larger default text, system fonts, relative type sizes and a saved **Text size** control from 90–200%. Layouts grow and stack to keep controls usable at larger sizes.
- Linux settings/profiles use XDG config; engines and default models use XDG data; logs/history use XDG state; temporary caches use XDG cache. The default model folder is `$XDG_DATA_HOME/llama-launcher/models`, normally `~/.local/share/llama-launcher/models`. Choose **Models → Models folder** to save a custom location.
- Existing saved folder choices are preserved. One-time migration copies old metadata/backups without deleting originals or moving large models/runtimes. Retained old runtimes are external imports. Keep the old `data` folder while those files are still referenced.
- Correct recovery filenames so damaged records can be restored after migration.
- Windows portable packaging includes Fixed WebView2 and app-local VC runtime DLLs, temporary runtime cleanup and saved-data recovery restart support.
- Refresh compatible Rust/npm/build dependencies and verified Microsoft runtimes. The Linux baseline is Debian 13, WebKitGTK engine 2.54.0+ and OpenSSL 3.5.7+. Detailed package versions, notices and the offline settings guide are included.

## Validation and scope

Publication requires both platform builds, backend tests, browser regressions including saved model folders/download destinations and 100–200% text, the Windows portable GUI/cleanup/relaunch checks, and AppImage startup on clean Debian 13 and Ubuntu 26.04 without system WebKitGTK/FUSE.

The Windows executable is unsigned. GPU/driver coverage and long-running workloads remain dependent on the selected llama.cpp runtime and hardware. Hugging Face browser sign-in requires a registered public OAuth client ID; read-token authentication is available. Existing release tags and assets are preserved.
