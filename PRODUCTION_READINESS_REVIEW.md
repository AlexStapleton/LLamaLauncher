# Production readiness and optimization review

**Status correction — 2026-10-16:** This document preserves historical local review evidence. Release 1.1 subsequently passed Windows and native Ubuntu 22.04 CI, including the optimized glib regression. References below to Linux execution being outstanding describe the earlier review, not current 1.1 CI. The 1.1 Linux ZIP still required host WebKitGTK; a user encountered that missing dependency. The 1.1.1 packaging fix replaces public ZIPs with a Windows setup EXE and a WebKitGTK-bundled AppImage, with installer upgrade/uninstall and clean Ubuntu 24.04 AppImage checks required before publication. Broader GPU/driver, clean Windows, OAuth and soak gates remain open.


Reviewed: 2026-10-14. Scope: the current local LLamaLauncher working tree, including the recently implemented features. The working tree is not a published release.

LAN controls follow-up: listen address and port now appear in Launch with local-only/LAN presets and a custom interface address. Eleven frontend tests and the browser regressions pass. A native check verified wildcard binding, health access through the computer's IPv4 interface, and inference after binding that specific IP. No firewall rules were changed and no second device was tested. The benchmark measurements and package/binary hashes below identify the earlier review build, before this UI follow-up.

## Verdict

**Treat this as a Windows beta/test build. It is not ready for an unrestricted production release. Linux readiness is unproven.**

The follow-up implementation addresses the identified code defects and scaling problems. Windows native and browser regressions pass. Remaining production gates concern Linux execution, the hardware/driver matrix, clean-machine deployment, registered OAuth and longer storage/inference stress. These checks cannot be inferred from the Windows results.

The Rust/Tauri frontend split is not the main problem. Rust owns processes, filesystem operations and networking; the web frontend owns the interface; Vite builds its assets. llama.cpp performs inference in a separate process. Rewriting the interface or replacing Rust would not fix the identified bottlenecks. Keep this architecture and improve its boundaries.

Generated binaries, ZIPs, raw benchmark files and screenshots referenced below are local outputs excluded from Git. Reproduce them with the documented test/build commands or retrieve the artifacts from a verified GitHub Actions run.

## Review scope and evidence

Reviewed the active Rust modules, native command validation, frontend state/IPC/profile/MCP/features code, dependency manifests, security configuration, build scripts, portable packaging and prepared workflow. Deprecated Python code and the protected Tauri configuration were not changed.

| Area | Source assessment | Execution evidence |
|---|---|---|
| Launch and process ownership | Validation, argument construction, readiness, stop, snapshots, Windows jobs and Unix process groups | Real Windows CPU launch, inference, stop, router load/unload; lifecycle tests |
| Models | Recursive scan, bounded GGUF metadata, cache invalidation and estimates | Scan/parser tests and a real downloaded GGUF |
| Persistence | Private atomic writes, migration, portable paths, profiles and account/download/preferences loaders | Windows DPAPI/profile tests, cross-process lock test, copied-folder relocation test |
| Networking | MCP HTTP/stdio framing, credentials, Hub API, downloads, SSE and cancellation | Local MCP fixtures, public Hub and official release downloads, real local inference |
| Interface | Navigation, settings, profile operations, async state changes and rendering costs | Browser smoke checks and five-run browser benchmark |
| Distribution | Windows ZIP and Linux AppImage scripts, fixed WebView2 inputs, workflow | Windows native executable tested; current Linux build/clean-machine matrix not executed |

No live MCP tools were invoked. Native tests used isolated data and public test models, not the user's saved credentials.

## Fixes made during this review

| Finding | Change | Verification or limit |
|---|---|---|
| Networking dependency had an HTTP/2 advisory | Updated reqwest; the lockfile now resolves reqwest 0.13.5 and h2 0.4.20 | RustSec vulnerability count fell from 1 to 0; networking fixtures and real download/inference tests passed |
| Two GUI instances could replace each other's saved data | Added an OS file lock per data directory, held for the GUI lifetime; the MCP configuration CLI also takes it | A separate process is excluded while the handle is held and can acquire it after release |
| `-hft=...` leaked into command previews; additional structured secret values could leak | Redacted the inline short alias and every value of recognized structured secret flags | Regression tests preserve ordinary arguments while excluding fixture secrets; arbitrary credential surfaces still need review below |
| Inference continued waiting after `[DONE]`, and truncated streams appeared successful | Exit on the completion marker; report EOF before the marker as failure | Tests cover a connection kept open after DONE, truncation, fragmented output and cancellation |
| Every owned-server request created a new HTTP client | Reuse one pooled client for health/API/metrics/model/inference requests | Real native requests passed; a numerical latency improvement was not measured |
| Large download writes and file hashing ran on the async executor | Use asynchronous filesystem writes/flushes for model and runtime archives; move whole-file SHA256 verification to a blocking worker | Real verified downloads passed; multi-gigabyte throughput and disk-stall responsiveness still need measurement |
| Runtime deletion could race server startup after its initial check | Hold the server-manager lock through removal in a blocking worker | Build/native install-remove tests passed; broader profile/remove races still need stress coverage |
| An older Hub search response could replace newer search results | Discard superseded search responses | Typecheck/UI smoke passed; dedicated delayed-response stress coverage remains desirable |
| Help output and MCP snapshots escaped the portable data folder | Put these temporary files under `data/cache` | Rebuilt/native checks and portable storage checks passed |
| Browser performance harness omitted the new storage commands | Repair its mock and use the actual run timestamp | Five measurements completed; these remain mocked-backend measurements |

The networking advisory concerns unbounded empty HTTP/2 DATA frames and is patched at h2 0.4.16 or later. [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2026-0258.html).

## Follow-up fixes implemented

| Finding | Result | Verification |
|---|---|---|
| R1 logs | Independent draining relays, 10 MiB per file, two backups per stream, 60 MiB combined cap; scrub known credentials before writes | Rotation storm, split-secret relay and write-failure draining regressions; native repeated start/stop |
| R2 saved records | Version envelope, matching size limits, protected backups, write-blocked damaged/future records and explicit retry/restore/reset UI; originals retained | Damaged/future/oversized records, denied backup publication, explicit restore and original-byte preservation; portable migration tests |
| R3 backend selection | Validate executable and actual GPU device rows; prefer CUDA candidates on NVIDIA, try Vulkan, fall back to verified CPU; record probe outputs and upstream archive digests | GPU-row parser, real selected-runtime inference, explicit failure/retry; broad GPU/driver matrix remains a release gate |
| R4 saves and close | Async worker persistence, serialized working saves, retained failed draft, visible retry, pending-save flush on close | Delayed/denied saves, rapid edits and close-before-save browser event regressions; native private writes |
| R5 downloads | Shared immutable plan and two workers; deduplicate/bound plans; aggregate progress; cross-process directory lease; finish complete partials locally; no-clobber atomic publication | 1,024-file plan bounds, duplicate rejection, offline completed-part finalization, unrelated-target preservation; real model download |
| R6 MCP | Freeze connection fields during operations and reject superseded results; profile/JSON load revokes old connections; 100-row tool pages | Delayed response after loading new settings; 10,000 tools with paging/search; native MCP fixtures execute no tools |
| R7 monitoring | Sample server descendants including model workers/helpers; label CPU sum and whole-device GPU readings; sample benchmark startup/250 ms intervals | Synthetic descendant tree, two simultaneously loaded router models and native monitoring-off checks; peaks remain sampled |
| R8 installs | Verified file manifest and protected receipt before publication; owned staging cleanup and startup/retry reconciliation; cancellable probes; serialize removal with jobs/profiles | Receipt rejects changed/extra files before execution, archive containment, native install/remove; native registry-write failure/retry test included |
| R9 sharing | Central filtering, explicit omission of custom executable/MCP arguments from public JSON, URL credential removal, known-secret log scrubbing and safe CSV names | Alternate/inline/multiple tokens, private stdio arguments, authenticated URLs, CSV formulas/separators; arbitrary unmarked free-text secrets are not semantically identifiable |
| R10 Linux dependency | Backport the upstream glib out-pointer fix while keeping Tauri/GTK versions compatible; verified source archive and 96-file integrity manifest | Source matches the documented fix; optimized iterator regression and FUSE-free AppImage smoke prepared; native Linux execution outstanding |

Other review fixes: stalled router SSE timeout/fallback, selected-runtime alias/capability validation and integer ranges, private auth loaded outside manager locks, worker-thread model finalization/probes, pooled Hub client, hidden-view rendering suppression, stable progress-row identity, visible listener failure/poll fallback, terminal-result deduplication, strict main/MCP TypeScript, explicit typed job results with runtime validation, and DOM/IPC checks, Unix directory flushing and 32-result benchmark retention. Packagers include generated dependency notices and an explicit unsigned-beta release policy.

The glib backport was compared against the official crate with SHA256 `edb0306fbad0ab5428b0ca674a23893db909a98582969c9b537be4ced78c505d`; only the documented mutable out-pointer source change differs. [Upstream fix](https://github.com/gtk-rs/gtk-rs-core/pull/1343), [advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html). RustSec no longer lists this package because it now has a local path source; that absence alone is not verification. The source comparison and prepared optimized Linux test are the relevant evidence.

## Original findings, in release priority order

The descriptions below record the problems found before the follow-up implementation. R1–R9 now have source fixes and regression coverage; R10 has a compatible source backport, with native Linux execution still outstanding. The current implementation and verification status appears in the follow-up table below. These original descriptions are retained to explain why the fixes were necessary.

### R1 — High: logs can exhaust storage

[Server launch](tauri-app/src/main.rs) connects stdout/stderr directly to files. Reading only the last 32 KiB in the interface limits UI memory, but does not limit those files. A long verbose server session can fill the portable drive. New launches also overwrite previous-session evidence.

Add bounded, rotating logs with an explicit total disk cap. A pipe relay must keep draining both streams even if writing fails, and must stop with the owned process tree. Verify a large log storm, a full/read-only disk and repeated start/stop; the server must remain responsive and storage must stay capped. This needs careful lifecycle work, not just truncating an actively written file.

### R2 — High: some damaged saved records are silently replaced

[Hub account loading](tauri-app/src/hub.rs), [download loading](tauri-app/src/downloads.rs), monitoring preferences and standalone MCP loading turn read/decode errors into defaults. Later writes can replace the original damaged record. Download saving also needs a matching size bound: loading is limited to 4 MiB, but saving does not enforce that limit. A valid, larger saved library can therefore become unreadable on the next launch.

Use explicit missing/loaded/damaged/unsupported-version states. Preserve the original, surface a recovery message, and require an intentional recovery/reset before replacement. Add protected backup/restore and consistent read/write size limits. Test malformed JSON, wrong Windows account protection, truncated files, oversized libraries, interrupted migration and future versions. The named profile library already handles several of these cases better and should provide the pattern.

### R3 — High: GPU selection does not establish compatibility

[Automatic selection](tauri-app/src/registry.rs) matches adapter brands and checks for a Vulkan loader file. That does not prove a usable device, required extensions or adequate drivers. Installation probes `--version`; [import](tauri-app/src/registry.rs) suppresses a failing device-list probe. There is no verified CPU fallback when the GPU backend is unusable. Automatic selection also does not compare CUDA against Vulkan on NVIDIA hardware.

Keep the user's Stable/Beta choice. Internally validate candidate backends with the installed executable, record the usable devices and fall back explicitly to a verified CPU build when needed. Only claim compatibility after the probe succeeds. Test CPU-only systems and representative NVIDIA, AMD and Intel devices, including missing/old drivers. Choose backend preferences using measured compatibility/performance, not brand matching alone.

### R4 — High: persistence can stall the GUI thread

[Working settings](tauri-app/src/features.rs) saves synchronously through DPAPI, disk write and `sync_all`. The frontend schedules these saves after editing. Monitoring preference saves and partial-download removal also perform disk work in synchronous native commands. Slow USB, network storage, antivirus or a stalled disk can make this interface stop responding. Tauri 1 executes ordinary synchronous commands on its main thread. [Tauri command documentation](https://v1.tauri.app/v1/guides/features/command/#async-commands).

Move these operations into blocking workers behind async commands, keep writes serialized, retain failed pending saves for explicit retry, and show save state. The current 250 ms debounce also has no coordinated close-time flush, so the final edit can be lost on a rapid exit. Test delayed writes, denied writes, fast edits/profile switches and close immediately after an edit.

### R5 — High for large selections: download scheduling scales poorly

[Download launch](tauri-app/src/downloads.rs) creates a task per file and clones the entire plan into each task. Each plan contains the full file list: retained metadata scales quadratically with the number of selected files. The semaphore limits transfers to two, but does not limit the already allocated waiting tasks and copies. The allowed selection is up to 1,024 files.

Use a shared immutable plan and a bounded two-worker queue. Reject/deduplicate repeated filenames before starting. Aggregate per-file progress into one job total; currently parallel files overwrite that job's completed/total/speed values, so displayed progress and ETA can jump. Verify many-file selections, queued cancellation and realistic large shards with peak RSS, response latency and disk throughput measurements.

### R6 — High: MCP connection results can replace newer settings

[MCP action](tauri-app/src/frontend/mcp-ui.ts) captures a profile, waits for a connection/test and applies its result. The fields remain editable during that wait. Editing increments a revision, but the action result does not check it; only the heartbeat checks revisions. An old response can restore old connection settings and show the wrong tool catalog.

Guard action results with the configuration/connection generation, and cancel/revoke superseded connections without replacing newer working settings. Alternatively freeze affected fields for the operation, while still handling profile changes correctly. Test a deliberately delayed connection while editing the endpoint/token or loading another profile; stale tools/settings must never become current.

### R7 — High for router accuracy: resource readings omit model workers

[Sampling](tauri-app/src/monitor.rs) refreshes only the root server PID. Router model weights and CPU work live in worker processes, so the dashboard's Server RAM/CPU can substantially understate actual use. Existing single-model integration checks do not establish correct router accounting.

Track and sample the owned server process tree, retaining CPU deltas for new/disappearing workers. Label whether figures include bridge/helpers. Keep GPU figures clearly marked as whole-device usage. Verify two loaded router models, worker exit/reload and monitoring off. Benchmark peak-memory samples also miss startup and very brief peaks; label them as sampled peaks until measurement improves.

### R8 — High: runtime installation is not a complete transaction

[Installation](tauri-app/src/registry.rs) moves extracted files into their final folder before import/registry persistence finishes. A subsequent probe or registry-write failure leaves an unregistered final folder. Retrying then refuses that folder. A crash can also leave staging directories because destructor cleanup does not run.

Add a durable installation manifest/receipt and recoverable stages. Either roll back publication safely or reconcile a verified owned install at startup. Persist upstream asset identity and digests in the artifact record, not only the transient job result. Test permission/disk errors after extraction and publication, cancellation during probes, process termination and retry. Preserve unrelated files throughout recovery.

### R9 — High for sharing: credential filtering is incomplete

[Configuration export](tauri-app/src/main.rs) removes standard credential fields and recognized server flags, but retains custom MCP stdio arguments. A token supplied through a custom `--token` argument would survive export. Known-flag filters also cannot reliably identify credentials embedded in arbitrary URLs/arguments. Raw server logs and the inline failure excerpt do not provide complete credential scrubbing.

Define a shareable configuration DTO with explicit sensitive-field omission, including a deliberate policy for custom MCP arguments. Centralize display/export redaction and test alternate flags, multiple values, token-bearing URLs, stdio arguments and environment credentials. Standard-field tests passing must not be presented as a guarantee that every export/log is free of secrets.

### R10 — Linux release gate: dependency soundness and compatibility

RustSec still reports an unsound `glib::VariantStrIter` implementation in glib 0.15.12. The Linux dependency chain uses that version through Tauri's windowing/GTK stack; it is not part of the Windows native runtime. RustSec lists glib 0.20.0 as patched. Actual application reachability has not been established. [glib advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).

Resolve the Linux dependency finding through a verified compatible stack or reviewed backport. Do not force a major glib version underneath an incompatible GTK dependency. Then build and run the actual AppImage on the supported distribution matrix, including FUSE-free startup. A Windows pass cannot close this gate.

## Other findings recorded before fixes

- **Router event freshness:** the SSE watcher has no idle/heartbeat timeout. An open, stalled connection can keep `connected=true` and suppress the interface's fallback model poll. Add freshness handling and test dropped/stalled streams.
- **Option validation:** integer-valued structured options are parsed as floating-point numbers, aliases/custom arguments do not receive equivalent range checks, and router remote-source conflicts are checked incompletely for custom arguments. Normalize aliases and validate the effective argument set against the selected executable's capabilities. Preserve support for version-specific flags.
- **Interface work while hidden:** telemetry redraws hidden charts; progress rebuilds the entire job list and its controls; MCP discovery can build thousands of tool rows. Render only visible changes, retain stable row/button identity, and paginate or virtualize large catalogs. Test keyboard focus during progress updates and a 10,000-tool fixture.
- **Startup resilience:** initial preferences/history failures can abort initialization before normal navigation is bound. Listener registration errors are swallowed. Display a usable degraded interface with a clear service error and retry action.
- **Remaining filesystem work:** archive dependency copies, registry probes in benchmarks and several persistence/metadata operations still need worker-thread review. Hub requests also construct clients repeatedly; pool clients where their TLS/redirect/auth policies are compatible.
- **CSV export:** profile names are only quote-escaped. Formula-like names remain active when opened in spreadsheet software. Add an explicit spreadsheet-safe export policy and tests for formula prefixes/separators. Quoting alone is insufficient. [OWASP CSV injection guidance](https://community.owasp.org/attacks/CSV_Injection).
- **Type coverage:** strict TypeScript covers the typed modules, but `checkJs` is false and the main application/MCP logic remains unchecked JavaScript. Incrementally type those modules and use explicit DTOs for job results instead of broad records/casts.
- **Durability and retention:** atomic replacement is useful, but Unix directory durability, schema/version handling for all records, benchmark retention and interrupted-download finalization need fault-injection coverage.

## Measured performance

Final run: five fresh pages in a warm headless Chromium browser, mocked backend, Windows/Ryzen 7 5800X, Node 22.23.3. Retained JS heap was measured after forced garbage collection. Raw samples are in `tauri-app/artifacts/gui-benchmark.json` (local output, not committed).

| Measurement | Median | Largest of five |
|---|---:|---:|
| Page ready | 76.84 ms | 102.84 ms |
| Open All options | 187.54 ms | 210.84 ms |
| Search options | 129.81 ms | 133.72 ms |
| Render batches during simulated typing burst | 1 | 1 |
| Retained JS heap | 2,764,824 bytes | 2,765,540 bytes |

These numbers suggest that the ordinary option catalog is reasonably small and responsive on this machine. They exclude native startup, WebView2 process memory, filesystem waits, real network latency and llama.cpp memory. An earlier run was slower during other work; this variation is not evidence of a measured code speedup.

The frontend output is approximately 144 kB JavaScript (41 kB gzip), 20 kB CSS and 15 kB HTML. The current Windows executable is 3,951,616 bytes, excluding WebView2 and the engine. Packaging size is primarily a separate question from launcher responsiveness.

The release profile uses size optimization, LTO and one codegen unit. There is no measured justification for changing `opt-level="s"` to `3`. Most inference throughput comes from llama.cpp, model/backend selection and runtime settings. Benchmark those independently from launcher UI performance. Memory estimates are explicitly approximate and must not be used as guaranteed fit checks.

### Native resource baseline

An early five-second Windows idle sample with monitoring disabled and the WebView2 debug harness attached measured root launcher RAM 31 MiB and the complete launcher/WebView2 tree 374 MiB. CPU used 3.46% of one core over that short interval. This includes browser initialization/background activity and diagnostic harness effects; it is not a steady-state performance budget or a before/after speedup claim. Read/write counters and raw samples are in artifacts/native-idle-resources.json.

With two small CPU router models loaded, server-tree telemetry reported 350.1 MiB; an independent Windows process snapshot measured root-router RAM 16.1 MiB. This demonstrates that the reported memory includes workers. The snapshots are asynchronous, so their byte totals need not match exactly. artifacts/router-memory-accounting.json retains the measurements. Multi-gigabyte throughput, large-shard RSS and an extended soak remain unexecuted gates.

## Dependency audit

- npm audit: **0 reported vulnerabilities**.
- RustSec before the dependency update: **1 vulnerability**, h2 0.3.27.
- RustSec after the follow-up: **0 vulnerabilities**, **4 informational warnings**: fxhash 0.2.1, instant 0.1.13 and proc-macro-error 1.0.4 are unmaintained; rand 0.7.3 has a soundness advisory. The locally backported glib source is assessed separately above.
- rand reaches this application through phf generation in the older Tauri dependency tree. Its advisory requires specific custom-logger/RNG conditions; those conditions were not demonstrated here. Do not equate its presence with a demonstrated launcher exploit. [rand advisory](https://rustsec.org/advisories/RUSTSEC-2026-0097.html).

Audit tool: cargo-audit 0.22.2. RustSec database commit: `edb58f5974b83eb8302332821cb6d389777aa351`, containing 1,329 advisories. Full before/after audit JSON and npm JSON are under `tauri-app/artifacts/`. Zero reported vulnerabilities is not a complete security assessment. The rand 0.7.3 chain reaches PHF code generation through phf_generator/phf_codegen/phf_macros; its inspected generator uses seeded SmallRng, not ThreadRng. The advisory requires a reentrant custom logger using ThreadRng during reseeding, and the application has no such custom logger. This narrows the known reachability; it does not erase the advisory. The three unmaintained crates are transitive Tauri dependencies and remain maintenance risks rather than silently ignored findings. Evidence: artifacts/rand-reachability.txt and the current audit JSON.

## Verification completed

| Check | Result |
|---|---|
| Frontend type/interface check and production build | Passed |
| Frontend unit tests | 9 passed |
| Rust formatting | Passed |
| Rust Clippy, all targets, locked dependencies, warnings denied | Passed; the restricted filesystem emitted incremental-cache hard-link/copy diagnostics |
| Native Rust release tests | 58 passed; 3 fixture entry points marked ignored and exercised by their parent tests |
| Windows MSVC native release build | Passed |
| Browser UI smoke | Passed, mocked backend/native event bridge, including delayed MCP, 10,000 tools, focus retention, failed saves and close flushing |
| Real Windows integration | Passed: GUI model selection, CPU inference, metrics, monitoring off, public Hub search/file selection/verified download, benchmark warm-up plus 3 measured runs, official runtime installation/removal, registry-write failure with receipt-based retry, Stable/Beta planning, two loaded router models with worker memory, and three extra start/stop cycles |
| Fresh portable package | Passed: packaged Fixed WebView2 resolves automatically; fresh GUI initializes; data is beside EXE; archive matches tested binary and excludes developer data |
| Native portable storage test | Passed: next-to-EXE location, folder relocation, managed path rebasing, WebView data location and legacy records preserved |

The first restricted native-test run could not perform localhost/private-file fixture operations. The permitted native run passed all tests. Linux builds and clean-machine tests were not executed. The prepared GitHub workflow was inspected, including the existence of its action refs, but was not dispatched.

Reviewed native executable: `tauri-app/target/release/llama-server-configurator.exe` (local output, not committed).

SHA256: `9588a7a04484bfff8ce24d19598201d0fc487dd74f2b66bf6d81e4daa32ba0b1`.

Fresh unsigned Windows beta package: `tauri-app/artifacts/LLamaLauncher-Windows-x64-20261014-170246.zip` (local output, not committed). ZIP SHA256: `aad392a234d816df4bc4d601400cad603e6844bba7553eb388676507b0f302e9`. The archive's EXE hash matches the verified binary; developer data/model weights are absent and dependency notices are included. A fresh packaged GUI passed startup using its own Fixed WebView2 with no runtime/data overrides, and created data beside the EXE. This is a same-machine check, not a clean Windows VM result. Earlier ZIPs retain their original contents. These measurements were made locally before publication; no remote workflow execution is implied.

## Release sequence and acceptance gates

1. R1–R9 source fixes and targeted regressions are implemented. Retain the broader physical disk-exhaustion, interruption and long-session tests as acceptance gates.
2. Resolve/document every dependency advisory with target and reachability evidence; complete the Linux dependency decision and native builds.
3. Measure native idle/active CPU, whole process-tree RSS, disk IO and interaction latency with monitoring on/off, simultaneous multi-gigabyte downloads and large catalogs. Set budgets from those baselines. Do not substitute browser heap measurements for whole-application memory.
4. Run clean Windows x64 tests without a preinstalled browser/runtime, plus the supported Linux/AppImage distribution and GPU/driver matrix. Verify offline startup, bad network/authentication, cancellation, disk exhaustion, interrupted updates and profile recovery.
5. Run an extended inference/router/download soak with repeated starts/stops and inspect residual processes, handle counts, memory and disk growth. Exercise registered browser OAuth before presenting that sign-in path as fully validated; token/public access testing does not establish OAuth correctness.
6. Produce a fresh package from the tested revision. Verify package checksums, dependency notices and fixed-WebView2 servicing. The current executable is unsigned; establish the signing/release policy and an update procedure that preserves `data`. Run the prepared workflow remotely and retain its evidence.

Production sign-off should follow those gates. The current successful Windows tests support continued beta testing, not a claim that every feature, platform and performance path is finished.

## Focused optimization pass — 2026-10-15

Parsed options now retain their resolved guide and are indexed by primary tab, avoiding repeated classification and whole-catalog filtering on narrower pages. Hidden option grids are cleared without building controls, unchanged view state skips DOM replacement, and identical executable help keeps the current catalog/controls. Search callbacks from departed pages no longer redraw the current page. No detached DOM cache was added. Changed help still replaces the catalog, including new unreviewed options; settings/profile changes invalidate the view revision.

Expanded browser regressions verify zero hidden Launch cards, control preservation when identical help finishes after leaving Launch, recognition of changed help, restoration of the original catalog, and zero Models mutations from a departed search. Five-run warm mocked-Chromium medians before/after: ready 86.80/90.79 ms, all-options interaction 158.38/158.75 ms, debounced search 120.92/123.67 ms; one render per typing burst in both. Retained browser JS heap was 3.04/3.11 MB. These timings show no meaningful general speed gain. The demonstrated benefit is removal of redundant work and edit disruption, not faster model inference. Raw reports are `artifacts/gui-benchmark-optimization-before.json` and `artifacts/gui-benchmark-optimization-after.json`.

Verification passed 15 frontend tests, strict type/interface checks, the complete browser suite, formatting, Clippy, 60 native release tests and a Windows release build. Fresh package `tauri-app/artifacts/LLamaLauncher-Windows-x64-20261015-064124.zip` has SHA256 `02e6ffd50bbdd5d1c0aefc2b94e85022016a7270b2928b39f6ad1a4ea1eba78d`; its EXE matches the release binary, SHA256 `9a0cc5572db7164ae01b8059cb10f39e5e587619a2b64d303fa848150253950d`. The completed archive excludes private/developer data and includes the offline guide/notices. A fresh staged GUI passed packaged WebView2 startup, zero hidden Launch settings cards, expanded cache guidance, native documentation URL validation and next-to-EXE storage. No user desktop copy was replaced. This is an unsigned Windows beta tested on the development machine; Linux/clean-machine gates above remain open.

## Settings-placement and documentation follow-up — 2026-10-15

All 257 bundled option groups now have explicit primary homes and reviewed, authored explanations in `option-guide.js`. Context/cache and chat/output have dedicated Settings tabs; model sources/adapters/media are in Models, native tools/MCP in Connections, and logging beside Activity → Logs. Draft-model CPU/GPU/cache controls stay with speculative decoding. The previous keyword matching misplaced chat templates, `--no-host` and draft controls. Dotted preset names now survive both frontend parsing and native capability validation.

The [complete settings reference](LLAMA_SETTINGS_GUIDE.md) is generated from GUI metadata. It records the official upstream source revision, describes increasing/decreasing and enabling/disabling each option, and distinguishes conditional storage calculations from measured throughput. Known examples include dense KV context scaling, cache-component precision ratios, and illustrative GPU tensor placement. Original executable syntax/defaults remain authoritative; unknown options are explicitly unreviewed. Informational/removed options are reference cards, with a way to remove existing saved overrides.

Follow-up checks passed: 15 frontend unit checks, strict type/interface checks, production frontend build, 60 native Rust release tests (plus 3 subprocess fixture entry points), formatting, Clippy and the expanded browser regression suite. Documentation opener URLs are restricted to official llama.cpp GitHub source pages. Browser tests use a mocked backend; native unit tests cover validation separately. A five-run warm Chromium benchmark with the guide loaded measured median ready time 90.19 ms, all-options render 167.20 ms, search 123.54 ms including debounce, one render per typing burst and about 3.04 MB retained JS heap. These are frontend measurements, not whole-process memory or model throughput. Earlier hashes and measurements above refer to earlier builds; native Linux and clean-machine gates remain open.

Fresh Windows settings build: `tauri-app/artifacts/LLamaLauncher-Windows-x64-20261015-004239.zip`, SHA256 `b85a48c0efc888abfa7852c99606b86dfb8e0e37984becfa424658f882b568eb`. Its EXE matches the release output, SHA256 `8435cea1662e3d2dd15cb0a43ef440010f61f314fe3343e13ff87904d7a90f6c`. Archive inspection confirmed the offline settings guide, dependency notices and absence of private/developer data. After the ZIP was completed, a fresh staged copy passed native packaged-WebView2 startup, next-to-EXE storage, six-part expanded cache help and native documentation URL rejection. It used isolated legacy context and did not invoke MCP tools or change the user's desktop copy. This remains a same-machine unsigned beta check.
