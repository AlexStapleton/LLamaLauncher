# Windows desktop application

Extract the complete portable ZIP and run `llama-server-configurator.exe`. Keep its private `WebView2` folder and VC runtime DLLs beside it. An ordinary developer build in `desktop-app` needs installed WebView2. Node.js and a development server are not required to run either build.

## Start a server

1. In **Launch**, download llama.cpp from its official GitHub releases using **Stable** or **Beta**, or browse to an existing Windows llama-server.exe. Candidate backends are probed, with verified CPU fallback.
2. In **Models**, select the models folder and a GGUF. Discovery includes subfolders and represents split models by their first shard. Use **Use for single-model launch** to return to Launch.
3. Configure options in **Settings**, then press **Start server**. **Stop server** stays in the fixed header and stops the app-owned process tree. Stop/restart to apply changed launch settings.

For LAN access, **Launch → Server address & port → Local network** selects `0.0.0.0`. Other devices use `http://<this PC's local IP>:8080` (or your chosen port). Enter a specific local IP in Listen address to bind only that interface. Restart after changes; **Only this computer** restores localhost. Allow llama-server on your private network if Windows Firewall prompts. API authentication options stay under Settings → API.

Router mode keeps the model picker visible and loads advertised models on demand. **Connections** manages Hugging Face accounts and HTTP/stdio MCP. MCP Connect enables router mode; Test initializes and discovers tools without executing them. Tool results are paged, searchable and expose expandable schemas.

## Settings and profiles

Settings use the selected executable's help catalog. Performance, GPU & memory, Context & cache, Sampling, Chat & output, API and Advanced have explicit option placement. Model/adapters/media options are in Models; native MCP/tool overrides are in Connections; logging is in Activity → Logs. All options searches across all categories.

Open **Help & resource impact** for purpose, direction, conditional magnitude examples, separate speed/VRAM/RAM effects, original executable help and official source links. All 257 bundled option groups have authored summaries; unknown runtime flags retain their own help and an unreviewed label. GPU weight layers and attention KV cache are separate controls. See the [full settings reference](../LLAMA_SETTINGS_GUIDE.md). Examples are not measured throughput or guaranteed allocations. Additional arguments use a JSON string array and are passed directly without a shell.

Named **Profiles** retain model/runtime references, all settings and credentials in private native storage. **Start server** uses current working settings, including unsaved changes. Public **Save JSON** omits authentication, custom executable/MCP arguments and URL authentication/query data; use named profiles for complete private preservation. Importing JSON revokes old MCP connections and retains missing model selections for explicit repair.

## Portable data and recovery

Profiles, working settings, accounts, preferences, engines, default downloads, logs and WebView state live in `data` beside the EXE. Keep that folder when updating. Managed references survive folder relocation; external paths may need relinking. Windows private records are DPAPI-protected for the same Windows account.

**Launch → Saved data & recovery** shows failed saves and damaged/future records. Retry retains pending edits; restore/reset preserves original protected bytes and restarts. Closing waits for pending saves. Server logs rotate within a combined 60 MiB cap and scrub known credentials. **Activity** includes logs, tasks and hardware monitoring; polling is adjustable from 1–60 seconds or completely off. Router CPU/RAM includes model workers/helpers; GPU figures are whole-device usage and benchmark peaks are sampled.

## Build and validation

Follow [HOW_TO_BUILD_WINDOWS.md](../HOW_TO_BUILD_WINDOWS.md) and [PORTABLE_RELEASES.md](../PORTABLE_RELEASES.md). Windows checks passed unit/interface tests, real inference/downloads, profile benchmarks, two loaded router models, install recovery, portable relocation and packaged WebView2 startup. See [the current readiness review](../PRODUCTION_READINESS_REVIEW.md).

Current distributions are unsigned beta builds. Local success does not establish clean-machine, all-GPU or Linux readiness. GitHub build/packaging workflows are prepared; their execution must be verified separately.
