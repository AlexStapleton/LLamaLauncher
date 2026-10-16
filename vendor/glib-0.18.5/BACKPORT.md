# GLib 0.18.5 compatibility backport

Official crates.io glib 0.18.5 archive SHA256: `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`.

Tauri 2's GTK 3 dependency graph still requires glib 0.18. Its VariantStrIter has [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), fixed upstream in 0.20. Backport only the mutable out-pointer change from [upstream PR 1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343): `let mut p` and `&mut p` in src/variant_iter.rs. All other upstream files are unmodified. The manifest records the packaged source file hashes; the optimized Linux application regression exercises both iterator directions and consuming last(). Native GLib is supplied by the system/AppImage and is distinct from this Rust binding version.

Keep upstream licenses and source provenance. Replace with a fixed compatible upstream release when the framework's GTK graph supports it. Absence from cargo audit for a path dependency is not security evidence; verify the source and native regression.
