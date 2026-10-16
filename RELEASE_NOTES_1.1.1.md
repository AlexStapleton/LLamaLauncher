# LLamaLauncher 1.1.1

This packaging fix replaces the 1.1 ZIP downloads with a Windows setup executable and a Linux AppImage. The old Linux raw executable needed host WebKitGTK 4.0 and could fail with `libwebkit2gtk-4.0.so.37: cannot open shared object file`. The application is migrated to stable Tauri 2.12.4, with current Tauri JavaScript/CLI APIs and narrowly scoped event permissions. The AppImage bundles WebKitGTK 4.1/libsoup 3 and its process helpers; the build rejects a WebKit engine older than the reviewed 2.54 stable line. The WebKit API number (4.1) is different from the engine version (2.54.x). Native package versions are recorded in LINUX_PACKAGE_VERSIONS.txt inside the image. GTK 3 remains required by Tauri. Its compatible GLib 0.18.5 Rust binding retains a verified source backport for RUSTSEC-2024-0429.

## Windows

Run `LLamaLauncher-1.1.1-Windows-x86_64-Setup.exe`. Setup includes private Fixed WebView2 and VC runtime DLLs, installs for the current account, and creates desktop/Start menu shortcuts. The default folder is `%LOCALAPPDATA%\Programs\LLamaLauncher`. Profiles, downloaded engines/models and settings remain in `data` beside the EXE. Updates and uninstall preserve data and unrelated files. Close the app and stop its server before updating. To retain an old portable installation, choose that folder during setup or copy its protected data folder to the new installation while closed, using the same Windows account.

## Linux

Download `LLamaLauncher-1.1.1-Linux-x86_64.AppImage` into a writable folder:

```sh
chmod +x LLamaLauncher-1.1.1-Linux-x86_64.AppImage
./LLamaLauncher-1.1.1-Linux-x86_64.AppImage
```

Without FUSE, run `./LLamaLauncher-1.1.1-Linux-x86_64.AppImage --appimage-extract-and-run`. No system WebKitGTK 4.0 installation is required. The baseline is Linux x86_64, glibc 2.39 or newer, with a desktop display; ARM and Alpine/musl are not supported by this asset. Keep `data` beside the AppImage when upgrading. Models and llama.cpp are downloaded separately in the application.

## Validation and status

Publication requires both platform source checks and native tests, a Windows installed-GUI check with bundled WebView2, upgrade/uninstall data preservation, verification of bundled WebKitGTK/helpers, and AppImage startup in a clean Ubuntu 24.04 container without system WebKitGTK or FUSE. Checksums accompany both assets; licenses and the offline settings guide are included.

This remains an unsigned beta. Broad GPU/driver coverage, clean Windows deployment, registered browser OAuth and extended soak testing remain open. Container startup does not establish compatibility with every Linux desktop or driver. Existing 1.1 release artifacts remain unchanged; use these 1.1.1 downloads for the packaging fix.
