# Optimization implementation and verification

October 13, 2026. Work is in the local LLamaLauncher checkout. No publication or remote workflow execution is implied by this report.

## Changes corresponding to the review

| Review item | Implemented change |
|---|---|
| Server lifecycle/readiness | One synchronized manager owns the process, worker tree and bridge snapshot. Background commands serialize transitions. Health checks distinguish loading from ready; state changes are emitted to the frontend. Snapshot and help-process guards clean up failure paths. |
| MCP memory bounds | Incremental framing rejects oversized unterminated lines before full allocation. Stdio queue is bounded to eight frames, message payloads to 4 MiB and total discovered tool JSON to 16 MiB. Profile and configuration reads are bounded too. |
| Polling/contention | Heartbeats return connection state/message only; network pings run without holding shared state. Generations prevent completed old pings/connections from replacing newer state. UI disconnect stays available during heartbeat. Server events replace frequent status polling, with a non-overlapping five-second fallback; hidden windows skip frontend polling. |
| Rendering | Collapsed category controls, option help and tool schemas are created on demand. Group expansion persists across rerenders. Searches are debounced, normalized search text and alias maps are cached, and unchanged log text is not rewritten. |
| Linux correctness | Paths retain Linux case/backslash semantics; previews use POSIX quoting; executable permission is checked; binary dialogs are not restricted to .exe. Owned Unix processes receive SIGTERM before forced cleanup. Readiness supports IPv6/multiple listen addresses, HTTPS and Unix sockets. |
| Configuration contract | Schema version 2 has one argv override representation. Legacy configuration fields migrate explicitly on import; future versions are rejected. Rust DTOs generate checked TypeScript interfaces. Typed IPC/platform modules start the incremental TypeScript migration; existing DOM modules remain JavaScript. |
| Help/scanning | Executable help is cached by canonical path/size/mtime with bounded output and cleanup guards. Scans skip unrelated files before canonicalization, cancel older requests, emit progress, and have a five-second cache. Refresh bypasses the scan cache. |
| MCP protocol | Incoming requests cannot be mistaken for replies with the same ID. Unsupported client requests receive an explicit JSON-RPC error; ping requests are answered. Observed tool-list notifications trigger rediscovery and an event. Bridge advertises empty client capabilities. |
| Build/release | Vite updated from 4 to 8; strict TypeScript contracts, stale-frontend protection, Rust formatting/Clippy, debug-profile CI tests, compilation cache, interface tests/measurements, Linux window smoke check and Windows/Linux artifact upload added. Native production scripts build assets before Rust and verify checks. Tauri remains on locked 1.x. |

Linux implementation and CI checks are prepared, but a real Linux build/window run is still pending. WSL and Docker are unavailable on this desktop. The workflow must run on a Linux host after publication before Linux support can be called verified.

## Measured results

Headless Chromium with mocked native calls, five fresh-page runs in a warm browser on this Windows desktop. Baseline frontend was extracted from the staged pre-optimization source; both frontends were bundled with the same Vite/browser versions. Retained heap was sampled after explicit garbage collection. These are small local samples, not inference or native WebView benchmarks.

| Median | Baseline | Updated |
|---|---:|---:|
| Page-ready wall time | 57.80 ms | 72.37 ms |
| All-options tab interaction | 109.64 ms | 104.27 ms |
| Single search interaction | 13.59 ms | 125.90 ms |
| Render batches during 16-character rapid typing | 16 | 1 |
| Retained JavaScript heap after GC | 2,518,384 bytes | 2,496,904 bytes |

The 100 ms debounce deliberately increases single-search latency while avoiding repeated rebuilds during rapid typing. Heap differences are small; these results do not justify claiming an overall RAM reduction or faster startup. Uncollected heap samples varied with GC timing and are not used to claim a memory improvement.

A separate native Windows run used an isolated profile with no model inference: window creation took 85 ms, seven owned processes had a summed working set of 355,393,536 bytes, and no CPU increment was measured over approximately five idle seconds. This is one run; window creation is not DOM readiness, and summed working sets may double-count shared pages. Repeat on representative Windows/Linux hardware before setting resource budgets.

The optional synthetic scan benchmark creates 20 folders and 2,040 small files, including 20 split GGUF models; it reports five native scan durations. It is not a benchmark of network-mounted model libraries or GGUF loading. MCP integration records native initialization/discovery elapsed time without executing tools.

The verified synthetic scan had a median of 7.39 ms (maximum 7.57 ms). The final native MCP probe took 144.43 ms for process startup, initialization and tool discovery in that run. Neither measurement has a comparable pre-change native baseline.

Raw measurement files and screenshots are under `tauri-app/artifacts/` and are intentionally excluded from Git. Run `npm run benchmark:gui`, `tests/native-benchmark.ps1`, or `node tests/scan-benchmark.mjs` to reproduce the corresponding workloads.

## Validation

Frontend unit tests cover every bundled option, alternate-version multi-argument syntax, path values, Linux path case and shell quoting. Strict generated-contract checks and frontend builds pass. Rust tests cover HTTP JSON/SSE/session/pagination, stdio lifecycle, framing bounds, protected profiles, configuration migration, model scans, process cleanup and readiness. The two ignored functions are subprocess fixtures invoked by the active tests, not skipped behavior. Headless interface checks separately verify controls, lazy help, themes, model population, MCP, configuration and responsive layout.

The real Docker AI MCP Gateway returned 49 tools through both native client and bridge. llama-server's router health/model-list check passed with an empty model directory. No tools were executed and no inference throughput/VRAM benchmark was performed.

## Deliberate limits and migration decisions

The [official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk) was evaluated as the broader protocol path. It supports a wider asynchronous transport/protocol surface than this application's narrow bridge. We retained the tested, bounded client for the gateway's negotiated 2024/2025 protocols rather than change Rust dependencies and the protected lockfile in this refactor. This is not a claim of full current-MCP compliance.

The bridge does not provide standalone GET notification streams, stream resumption, sampling, elicitation, roots, subscriptions or the newer 2026 protocol. Notifications seen during request streams are handled; idle HTTP tool-list changes require reconnecting if the server delivers them only over a separate stream. Timed-out exchanges end rather than transparently replay a potentially side-effecting tool call. HTTPS readiness requires a certificate trusted by the operating system; self-signed deployments need to establish that trust. Router readiness describes the router process, not every on-demand model's readiness.

A Rust-only GUI rewrite, broad framework introduction, full Tauri 2 migration and changes to release LTO/size tuning were not necessary to implement the review's incremental recommendations. Remaining JavaScript can migrate module by module using the generated contract. Keep production release flags until build-size/startup comparisons justify a different profile. npm audit currently reports zero advisories for the updated dependency graph; no independent Rust vulnerability audit is claimed.
