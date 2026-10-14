# Windows desktop application

This application now lives in the LLamaLauncher repository. Performance uses compact category groups with expandable help, and the header has a persistent light/dark toggle. The MCP tab supports authenticated HTTP or local stdio, Connect/Test/Disconnect, and searchable tool definitions. Connecting enables router mode. The live gateway and HTTP bridge each discovered 49 tools, and router startup was verified without loading a model. No tools were invoked.

For Pi/harness build instructions and troubleshooting, read [HOW_TO_BUILD_WINDOWS.md](../HOW_TO_BUILD_WINDOWS.md).

Run `desktop-app/llama-server-configurator.exe`, or use the **Llama Server Configurator** desktop shortcut. The frontend is embedded; Node.js and a development server are not needed to run it. Microsoft Edge WebView2 Runtime must be installed (it is present on the verified desktop).

1. Browse to your Windows `llama-server.exe`.
2. Select your models folder in Setup. GGUF models are discovered automatically, including subfolders. Select the desired model from the dropdown; split files are represented by their first shard.
3. Adjust settings and select **Start llama-server**. Use **Stop Server** before changing a running server's settings.

Settings persist locally. Save JSON and Load JSON use native file dialogs and include option overrides. Start and Stop remain visible in the fixed header; Stop is disabled when this app has no running server process. The launched process is stopped when the configurator exits normally. The Logs tab displays recent stdout/stderr and the application log directory.

Performance, GPU & memory, Sampling, API, and Advanced group the options from the selected executable. All options exposes the entire catalog, including aliases and negative flags. The bundled snapshot contains 257 option groups (414 flag names); selecting a different executable reads its own `--help` and refreshes the catalog. Search by flag or description, filter by category, or show only enabled overrides.

Enable Override before changing an option. Unchanged options use the executable's own defaults. Speed, VRAM and RAM guidance is available on every option card. These are qualitative effects, not measured allocations or benchmark results: architecture, quantization, context, backend and hardware determine the actual cost. Consult your executable's description for version-specific behavior; [official llama.cpp server documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) provides additional reference.

Model, listen address and port are configured in Setup/API. Remote sources and router mode in Advanced/All options take precedence over the local model. Additional arguments accept a JSON array for repeated flags and multi-value arguments. Everything is passed directly to the executable without shell concatenation. The PowerShell launch preview redacts API keys and access tokens.

API key and Hugging Face token overrides are not automatically retained in local browser storage. Save JSON explicitly if you want them included in a configuration file.

## Rebuild

Close the configurator, then run from this directory:

```powershell
.\build-windows.ps1
```

This builds the frontend, explicitly selects the installed Windows MSVC Rust toolchain, and refreshes the desktop-app executable. Do not use the default GNU toolchain unless MinGW is separately configured.

Validation: the frontend catalog and headless interface tests check all help entries, model population, pinned navigation, overrides, start/stop states, persistence, logs and responsive layout. Rust tests cover nested model scanning, split shards, safe argument passing, validation, serialization and actually stopping/reaping an owned subprocess. The initial native window/file picker were checked before this redesign; desktop UI automation was subsequently interrupted. Model inference requires your separate llama-server executable and GGUF model and has not been verified on this machine.
