# LLamaLauncher glib backport

Source: unmodified glib 0.15.12 crate from crates.io, SHA256
`edb0306fbad0ab5428b0ca674a23893db909a98582969c9b537be4ced78c505d`.

The only upstream Rust source change is in `src/variant_iter.rs`:
`VariantStrIter::impl_get` now declares its out-pointer mutable and passes
`&mut p` to `g_variant_get_child`. This backports the fix described in
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
and [upstream PR 1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343).

Cargo patches the existing glib dependency to this directory, preserving
Tauri 1 / GTK compatibility. No unrelated upstream APIs or version numbers
were changed. Upstream licenses are retained. The Linux release test in
the launcher exercises forward, reverse, nth and last string iteration.

RustSec may still flag the upstream version number. Record this source
backport when assessing that finding; do not silently ignore it or claim
the Linux regression test ran on Windows. Replace this fork with a compatible
maintained upstream release when available.
