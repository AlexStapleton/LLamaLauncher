# Changelog

## 1.1.0 — 2026-10-15

### Features

- Six-section interface, grouped settings, persistent dark mode and fixed launch controls.
- Explicit setting placement with dedicated Context & cache and Chat & output tabs; model/media, native MCP and logging controls alongside their related features. Speculative target/draft controls stay together.
- Reviewed help for all 257 bundled option groups, with direction, conditional magnitude examples, separate speed/VRAM/RAM effects and pinned official documentation links. Unknown version-specific flags retain executable help without invented estimates.
- Dotted model preset flags are preserved by frontend parsing and native capability validation.
- Named profiles containing model/runtime paths, all runtime settings and private connection credentials.
- Official llama.cpp Stable/Beta installation, automatic backend validation and CPU fallback.
- Hugging Face search, GGUF/shard downloads, read-token accounts and configurable public OAuth sign-in.
- Hardware preflight, adjustable monitoring with a complete off switch, inference tests, router model management and profile benchmarks.
- Portable data beside the executable/AppImage; Windows ZIP and Linux AppImage packaging workflows.

### Reliability and optimization

- Settings catalogs resolve guidance once and index options by destination tab. Hidden settings controls are not built on Launch/Activity/Profiles, unchanged executable help preserves active controls, identical views skip rendering, and departed searches cannot redraw another tab. Delayed/changed-help regressions pass; general interaction benchmark timings remain within run-to-run variation.
- Listen address and port are visible in Launch, with local-only/LAN presets, custom interface binding and a client address preview.

- Protected backups, explicit recovery for damaged/future saved records, matching size limits, async saves, retry and close flushing.
- Bounded, credential-scrubbing log relays that keep draining after write failures.
- Shared download plans, two workers, aggregate progress, directory leases, resumable complete-part finalization and no-clobber publication.
- Verified runtime receipts, interrupted-install reconciliation, cancellation and removal serialization.
- MCP generation checks, connection-field freezing, tool paging and lazy schemas.
- Model-worker resource accounting, sampled benchmark peaks, benchmark retention and stalled router-event fallback.
- Centralized public-export filtering, log/preview redaction and spreadsheet-safe CSV names.
- Strict main/MCP TypeScript, checked native contracts, typed job-result validation, stable progress controls and hidden-view rendering suppression.
- Networking dependency update and a provenance-checked compatible glib backport for Linux.

### Verification and release status

The settings follow-up passed 60 Rust tests, 15 frontend tests and expanded browser regressions. Earlier Windows verification also passed real inference/download/install/benchmark/router checks, install failure/retry, folder relocation and packaged WebView2 startup. Native tests used isolated data and did not execute MCP tools.

Current packages are unsigned beta builds. Linux/AppImage execution, the broader GPU/driver matrix, clean-machine deployment, registered OAuth and extended stress/soak checks remain release gates. See [the readiness review](PRODUCTION_READINESS_REVIEW.md). Historical measurements remain in the earlier optimization reports; GitHub Actions execution is separate from local verification.
