# LLamaLauncher architecture and optimization review

**Historical source review.** The [current readiness review](PRODUCTION_READINESS_REVIEW.md) and [changelog](CHANGELOG.md) describe the subsequent implementation. Source line numbers and JavaScript paths below refer to the original revision.

Reviewed October 13, 2026. This is a source review, not a performance benchmark. No application behavior was changed for this review. Windows builds and earlier integration checks are useful baseline evidence; Linux runtime behavior remains unverified.

The subsequent [implementation report](OPTIMIZATION_IMPLEMENTATION.md) records the applied fixes, benchmarks and remaining verification gaps. Findings below describe the pre-optimization source.

## Recommendation

Keep Rust/Tauri with a small web frontend. Move JavaScript to TypeScript incrementally, and give the Rust backend clearer module and state boundaries. A full rewrite is not justified by the evidence available.

The shipped application uses Rust plus browser JavaScript/HTML/CSS. Node/Vite and PowerShell/Bash are build tools, not additional application servers. The previous Python application is absent from this repository. The separate llama-server process performs inference; changing the launcher UI language will not directly improve its token throughput or reduce model VRAM.

Tauri deliberately combines Rust with an operating-system webview: https://v2.tauri.app/concept/architecture/ and https://v2.tauri.app/concept/process-model/ . A Rust-only GUI would remove the JavaScript boundary but replace the existing UI, accessibility behavior, styling and interface tests. Consider it only after measuring webview overhead and establishing a concrete requirement that this architecture cannot meet.

## Prioritized findings

### 1. Make server ownership and readiness explicit — high priority

Evidence: `src/main.rs:86`, `src/main.rs:715`, `src/main.rs:804`; `src/frontend/app.js:284`.

Process, process-tree guard and bridge snapshot are stored in three separate mutexes. Start/stop/status perform multiple independent mutations. This makes concurrent transitions and partial failures harder to reason about. Start reports success immediately after process creation, before a health check or model load completes. The interface consequently treats an existing process as a ready server.

Use one `ServerManager` owning a `ServerInstance { child, tree, snapshot, log_paths }`, with explicit Stopped/Starting/Loading/Ready/Stopping/Failed states. Serialize lifecycle transitions, emit state events, and perform bounded readiness checks against the actual configured endpoint. Distinguish router readiness from individual model readiness. Clean the snapshot on every failed start through an RAII owner; today failures after snapshot creation can leave private cache files behind.

Move synchronous start/stop filesystem and process waits off the UI command path. Add meaningful tests for concurrent start/stop/status, failed spawn/attachment, loading failures, and cleanup. Benefit: reliable controls and fewer apparent hangs, rather than faster inference.

### 2. Bound MCP allocations before reading — high priority

Evidence: `src/mcp.rs:173`, `src/mcp.rs:278`.

The HTTP SSE reader checks its total after `read_line` has allocated a complete line. The stdio reader similarly checks line size after `.lines()` has allocated it, then forwards parsed messages through an unbounded channel. A noisy or faulty server can consume memory before the nominal 4 MiB cap takes effect, or queue many notifications.

Use bounded incremental framing, a bounded message queue, maximum total tool/schema bytes and a notification policy. Add fixtures with oversized unterminated lines and sustained notification floods. Benefit: bounded RAM and predictable failure behavior.

### 3. Reduce polling payloads and lifecycle contention — medium/high priority

Evidence: `src/frontend/app.js:288`, `src/frontend/mcp-ui.js:81`, `src/main.rs:634`.

Server status is polled every 1.2 seconds. Logs are fetched again whenever their tab is active. MCP heartbeat locks the shared connection during a potentially 15-second network ping, then clones and returns the complete report including every tool schema; the interface only consumes connection state and message in that heartbeat.

Return a small heartbeat DTO, emit backend state changes, and retain slow fallback polling. Avoid overlapping polls and pause unnecessary work when hidden. Give MCP transport an owning worker or asynchronous task with explicit cancellation; a disconnect should not wait behind a stalled ping holding shared state. Send tool definitions only on discovery or a changed-list event. Benefit: less IPC serialization, allocation and waiting.

### 4. Render settings incrementally — medium priority

Evidence: `src/frontend/app.js:140`, `src/frontend/mcp-ui.js:34`.

Each settings search rebuilds all matching cards and listeners. Collapsed categories still construct all child controls and help. MCP search regenerates every matching row and pretty-prints every schema. Expanded sections are lost when rerendering.

Cache normalized search strings and alias lookup maps; debounce search; preserve group expansion; build help/schema content on first expansion. Update keyed rows or reuse category views. Add virtualization only if measured large-list latency warrants it. At 257 option groups, a framework rewrite is unnecessary without evidence. Benefit: smoother typing and less DOM work, especially on slower Linux machines.

### 5. Correct platform assumptions before claiming Linux support — high priority

Evidence: `src/frontend/app.js:69`, `src/frontend/app.js:72`; `src/process_tree.rs`.

Frontend path normalization lowercases every path, which conflates distinct Linux names such as `Model.gguf` and `model.gguf`. Command preview always produces PowerShell syntax. Unix process cleanup uses immediate SIGKILL; it does not give the server or MCP child a graceful shutdown period.

Compare canonical paths using platform semantics and have the backend supply platform-appropriate preview formatting. Implement graceful Unix SIGTERM followed by bounded SIGKILL fallback, and verify owned worker cleanup. Run actual Linux tests for case-sensitive paths, permissions, model discovery, WebKit rendering, and child cleanup. Windows success does not establish Linux correctness.

### 6. Establish one versioned configuration contract — medium/high priority

Evidence: `src/main.rs:23`; `src/frontend/app.js:18`, `src/frontend/app.js:47`; `src/frontend/option-catalog.js:1`.

Legacy typed fields coexist with generic argument overrides. Rust and JavaScript maintain overlapping assumptions; untyped IPC permits drift. Option metadata is inferred from prose help, and positional arity handling includes a special case for `START END` rather than a complete schema.

Introduce a versioned persisted configuration, explicit migrations, and typed IPC DTOs shared/generated from Rust. Migrate the frontend to TypeScript without changing its runtime architecture. Use explicit metadata overrides for ambiguous flags, with fixtures from multiple llama.cpp versions. Treat unknown options as structured argv, preserve values exactly, and keep validation in Rust. Benefit: fewer broken settings and easier maintenance.

### 7. Cache help and make scans cheaper — medium priority

Evidence: `src/main.rs:404`, `src/main.rs:344`.

Help is launched on initialization and refresh, written to a temporary file and read in full without a file-size cap. Scanning canonicalizes every entry, including unrelated files, before checking its extension. Scans have entry/depth bounds but no cancellation when the user selects a different folder; frontend request counters only discard stale results.

Cache parsed help by executable identity plus modification time/size (invalidate correctly), cap output, and guard temporary files/child cleanup on error. During scans, filter clearly irrelevant files before expensive canonicalization while retaining containment checks for directories and accepted models. Add cancellation, progress, and a modest scan cache with explicit refresh. Measure on a large local folder and network-mounted folder before/after.

### 8. Harden the MCP bridge contract — medium/high priority

Evidence: `src/mcp.rs:99` and its stdio exchange/bridge implementations.

The client supports the tested initialization, discovery, ping and request/reply flow. It is a hand-written protocol implementation, not demonstrated coverage of the entire negotiated protocol. In particular, unsolicited messages, server-initiated requests, changing tool lists, cancellation and resumable streams need explicit handling or explicit capability limitations.

Separate transport framing/session lifecycle from MCP semantics; evaluate a maintained Rust MCP SDK against bridge requirements before replacing working code. Keep integration fixtures for the Docker gateway and generic protocol fixtures. Do not advertise broader capability support than tested. Older negotiated Streamable HTTP revisions describe server-originated SSE messages: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2025-03-26/basic/transports.mdx .

### 9. Improve build and release feedback — medium priority

Evidence: `Cargo.toml` release profile; `.github/workflows/build.yml`; `tauri.conf.json` build hooks.

Release builds use full LTO, one codegen unit and size optimization. This favors distribution size over compilation speed; it has not been benchmarked against other profiles. CI currently runs release tests/builds without a Rust compilation cache, lint/type checks, interface smoke checks or uploaded distributable artifacts. Direct cargo builds can embed stale frontend assets unless the documented frontend build happens first.

Use debug-profile tests for fast CI feedback, a separate release build, Rust caching, formatting/Clippy, TypeScript checks and the GUI harness. Package Windows/Linux artifacts and run a Linux smoke test. Benchmark size/startup before changing release optimization flags. Centralize the build sequence through scripts or supported generated configuration; respect the repository rule against manually editing `tauri.conf.json`. Plan a separate Tauri/Vite/dependency upgrade with compatibility and security review, rather than combining it with functional refactoring.

## Measurement plan

Record cold/warm window-ready time, idle CPU and total launcher/webview RAM, search and tab-switch latency, model-scan time on representative folders, MCP discovery/heartbeat latency, bytes transferred through IPC, and release build time/size. Use repeated runs and report environment plus median/tail latency. No numerical improvement estimate is defensible yet.

Measure launcher costs separately from llama-server/model workers. For inference tuning, hold model and workload constant and measure prompt tokens/sec, generation tokens/sec, time to first token, peak RAM/VRAM and concurrency. Context length, KV cache precision, GPU offload, batch sizing and router model residency are where inference resource tradeoffs live; changing JavaScript to Rust does not itself change them.

## Proposed order

1. Baseline measurements; Linux path fixes; unified server lifecycle/readiness; bounded MCP framing and queues.
2. Smaller/event-driven status updates, incremental UI, cached/cancellable help and scans.
3. Typed IPC/versioned config and module extraction (`config`, `server`, `catalog`, `commands`, platform storage).
4. Verified Linux packaging/CI, dependency migration, broader MCP compatibility tests.

Preserve the existing tests and real gateway/router checks throughout. This is incremental improvement of a viable architecture, not a recommendation to rebuild the application in another language.
