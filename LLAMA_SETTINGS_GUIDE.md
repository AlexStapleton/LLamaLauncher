# llama.cpp settings guide

This reference contains the same authored summaries as the launcher for all 257 bundled option groups. Reviewed 2026-10-15 against official llama.cpp documentation and source at [commit 2130ca84764e](https://github.com/ggml-org/llama.cpp/tree/2130ca84764e75c7ae9106cec671caa73ddbe416).

The selected executable supplies syntax, supported flags and defaults. New options from a different version remain accessible in Advanced with their original help and an explicit unreviewed label. Unchanged settings use that executable's defaults. Existing overrides stay saved when their UI home changes. All options remains a searchable reference across every category. Information-only and removed flags are references, not normal launch toggles.

## Reading the estimates

GPU weight layers control weight/computation placement. KV cache holds attention state and has separate precision and placement controls. Doubling dense-attention context capacity roughly doubles its KV payload, not total application memory. Hybrid, recurrent and sliding-window architectures can behave differently.

For each 32 scalar values, f16 uses 64 bytes, q8_0 uses 34 bytes and q4_0 uses 18 bytes before other allocation overhead. These are about 53% and 28% of the f16 component payload. If K and V are equal-sized and only one changes to q8_0, combined payload is about 77%. Weight quantization is separate. These calculations are storage estimates, not throughput or quality promises.

Speed depends on model architecture, backend, hardware, prompt length, concurrency and acceptance rates. The examples state assumptions; benchmark prompt processing and generation separately with the same inputs. More threads, GPU layers or draft tokens need not be faster.

## Primary homes

| Location | Bundled option groups |
| --- | ---: |
| Settings → Advanced | 54 |
| Settings → Performance | 18 |
| Settings → Context & cache | 29 |
| Settings → Chat & output | 22 |
| Settings → GPU & memory | 18 |
| Activity → Logs → Native logging options | 10 |
| Models → Model sources, adapters & media options | 45 |
| Settings → Sampling | 29 |
| Settings → API | 24 |
| Launch | 2 |
| Connections → Advanced native tools & MCP options | 6 |

Model, models-directory, host and port cards redirect to the managed fields in Models or Launch. Other model options are configured in the expandable Models section. MCP and logging options have expandable sections alongside their related features. Speculative decoding keeps draft CPU, GPU, cache and proposal controls together in Advanced.

## Settings → Advanced

### Print executable help — `--help`

Group: Informational commands (exit). Aliases: `-h`, `--help`, `--usage`.

Show the installed binary's supported command-line options.

- **Change:** This command normally exits after printing; it is not an inference configuration to add to a launch.
- **How much:** Use Read executable options in Launch. The actual installed help determines valid syntax/defaults for all settings.
- **Speed:** Prints information or performs setup, often then exits; this is not a normal serving optimization.
- **VRAM:** Discovery may initialize a driver, but normally does not load model weights.
- **RAM:** Mostly temporary command output/completion data; no persistent model allocation.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Print runtime version — `--version`

Group: Informational commands (exit). Aliases: `--version`.

Show build/version information for the installed executable.

- **Change:** Normally prints then exits; choosing it as a server startup override would not start a normal serving session.
- **How much:** Use runtime/version information for compatibility checks. A larger version number is not a measured speed improvement.
- **Speed:** Prints information or performs setup, often then exits; this is not a normal serving optimization.
- **VRAM:** Discovery may initialize a driver, but normally does not load model weights.
- **RAM:** Mostly temporary command output/completion data; no persistent model allocation.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### List cached models — `--cache-list`

Group: Informational commands (exit). Aliases: `-cl`, `--cache-list`.

Print models already known to the runtime's download cache.

- **Change:** Lists cached files rather than loading all their weights, and normally exits.
- **How much:** This cache listing is separate from the launcher's Models folder scan. Cached disk files are not necessarily loaded models.
- **Speed:** Prints information or performs setup, often then exits; this is not a normal serving optimization.
- **VRAM:** Discovery may initialize a driver, but normally does not load model weights.
- **RAM:** Mostly temporary command output/completion data; no persistent model allocation.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Bash completion script — `--completion-bash`

Group: Informational commands (exit). Aliases: `--completion-bash`.

Print shell-completion setup for supported command options.

- **Change:** One-time shell convenience; does not change runtime computation.
- **How much:** Not useful as a long-running server launch flag. It prints script text rather than improving inference.
- **Speed:** Prints information or performs setup, often then exits; this is not a normal serving optimization.
- **VRAM:** Discovery may initialize a driver, but normally does not load model weights.
- **RAM:** Mostly temporary command output/completion data; no persistent model allocation.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### List available compute devices — `--list-devices`

Group: Informational commands (exit). Aliases: `--list-devices`.

Ask this executable which backend devices it can enumerate.

- **Change:** Normally prints then exits; it does not offload weights by itself.
- **How much:** A listed GPU establishes enumeration, not successful execution of every model/kernel. Use its identifiers for device settings.
- **Speed:** Prints information or performs setup, often then exits; this is not a normal serving optimization.
- **VRAM:** Discovery may initialize a driver, but normally does not load model weights.
- **RAM:** Mostly temporary command output/completion data; no persistent model allocation.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft key cache precision — `--spec-draft-type-k`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-type-k`, `-ctkd`, `--cache-type-k-draft`.

Draft model only, not the target model: Storage format for the attention key cache, separate from model-weight quantization.

- **Change:** Draft model only, not the target model: Lower precision reduces this cache's memory/bandwidth but can reduce quality or add conversion work; higher precision uses more memory. Backend/model support varies.
- **How much:** Raw-storage estimate: f16 → q8_0 uses about 53%; q4_0 about 28%. A 2 GiB K payload becomes roughly 1.06 GiB or 0.56 GiB, before alignment/metadata. This changes K only, not total application memory.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Draft value cache precision — `--spec-draft-type-v`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-type-v`, `-ctvd`, `--cache-type-v-draft`.

Draft model only, not the target model: Storage format for the attention value cache, separate from the key cache.

- **Change:** Draft model only, not the target model: Lower precision saves V-cache memory with possible quality/conversion costs; higher precision retains more detail and costs more memory. Quantized V can require compatible Flash Attention/backends.
- **How much:** If equal-sized K and V both change f16 → q8_0, a 2 GiB dense KV payload becomes roughly 1.06 GiB (53%); changing only V leaves about 1.53 GiB (77% combined), before overhead. q4_0 is about 28% per changed component; quality/speed are not guaranteed.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Draft model repository — `--spec-draft-hf`

Group: Speculation strategy. Aliases: `--spec-draft-hf`, `-hfd`, `-hfrd`, `--hf-repo-draft`.

Repository/quantization used to retrieve the proposal model.

- **Change:** Changing it selects a different draft with its own size/speed/acceptance tradeoff. It may download data at launch.
- **How much:** The target model is unchanged. Verify model/tokenizer compatibility and total memory for both models before increasing context/offload.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Draft generation CPU threads — `--spec-draft-threads`

Group: Speculation · Draft CPU threads. Aliases: `--spec-draft-threads`, `-td`, `--threads-draft`.

Draft model only, not the target model: How many CPU workers evaluate each newly generated token.

- **Change:** Draft model only, not the target model: Increase to use more cores; decrease if scheduling or memory bandwidth becomes the bottleneck. GPU-heavy runs can need fewer CPU threads.
- **How much:** Try 2 → 4 → 8 and compare generated tokens/s. More workers can be slower; start near physical-core count rather than assuming all logical threads are best.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Draft prompt CPU threads — `--spec-draft-threads-batch`

Group: Speculation · Draft CPU threads. Aliases: `--spec-draft-threads-batch`, `-tbd`, `--threads-batch-draft`.

Draft model only, not the target model: CPU workers used while reading the prompt in batches.

- **Change:** Draft model only, not the target model: More workers may shorten prompt ingestion; fewer leave CPU capacity for other work. This is separate from generation threads.
- **How much:** Changing 4 → 8 doubles the requested worker count, not guaranteed prompt speed or RAM. Compare prompt tokens/s and time to first token.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Draft CPU affinity mask — `--spec-draft-cpu-mask`

Group: Speculation · Draft CPU placement. Aliases: `--spec-draft-cpu-mask`, `-Cd`, `--cpu-mask-draft`.

Draft model only, not the target model: Choose the exact CPU cores the main model may use, using a hexadecimal bit mask.

- **Change:** Draft model only, not the target model: Adding bits permits more cores; removing bits restricts placement. A mask is not a thread-count setting.
- **How much:** For CPUs numbered 0–3, mask F permits those four cores. Choose according to your OS topology; a wider mask does not guarantee faster generation.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Draft CPU core range — `--spec-draft-cpu-range`

Group: Speculation · Draft CPU placement. Aliases: `--spec-draft-cpu-range`, `-Crd`, `--cpu-range-draft`.

Draft model only, not the target model: Restrict the main model to a numbered range of CPU cores.

- **Change:** Draft model only, not the target model: Widen the range to allow more cores; narrow it for isolation or locality. Also set a suitable thread count.
- **How much:** A range such as 0-3 allows four numbered CPUs; those may be logical siblings rather than four physical cores.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Strict draft CPU placement — `--spec-draft-cpu-strict`

Group: Speculation · Draft CPU placement. Aliases: `--spec-draft-cpu-strict`, `--cpu-strict-draft`.

Draft model only, not the target model: Control whether workers strictly follow the requested CPU placement.

- **Change:** Draft model only, not the target model: 1 enforces placement more strictly; 0 permits normal flexibility. Strict placement can help locality or hurt load balancing.
- **How much:** This is a policy switch, not a RAM or thread multiplier. Compare both on your actual multi-core/NUMA layout.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Draft scheduling priority — `--spec-draft-prio`

Group: Speculation · Draft scheduling. Aliases: `--spec-draft-prio`, `--prio-draft`.

Draft model only, not the target model: How strongly the operating system favors main-model CPU work.

- **Change:** Draft model only, not the target model: Higher priority favors this process under contention; lower priority makes it yield more readily to other applications.
- **How much:** The levels are discrete, not speed percentages. Realtime can make the desktop unresponsive; use normal unless measurements justify a change.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft busy polling — `--spec-draft-poll`

Group: Speculation · Draft scheduling. Aliases: `--spec-draft-poll`, `--poll-draft`.

Draft model only, not the target model: How actively workers spin while waiting for generation work.

- **Change:** Draft model only, not the target model: Increase for less waiting latency at the cost of CPU time/power; decrease to let idle workers sleep more.
- **How much:** The reviewed executable uses 0/1 for this waiting-policy switch. 1 enables polling and 0 disables it; neither means a specified percentage speedup.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft prompt affinity mask — `--spec-draft-cpu-mask-batch`

Group: Speculation · Draft CPU placement. Aliases: `--spec-draft-cpu-mask-batch`, `-Cbd`, `--cpu-mask-batch-draft`.

Draft model only, not the target model: Prompt-processing phase: Choose the exact CPU cores the main model may use, using a hexadecimal bit mask.

- **Change:** Draft model only, not the target model: Prompt-processing phase: Adding bits permits more cores; removing bits restricts placement. A mask is not a thread-count setting.
- **How much:** For CPUs numbered 0–3, mask F permits those four cores. Choose according to your OS topology; a wider mask does not guarantee faster generation.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Strict draft prompt placement — `--spec-draft-cpu-strict-batch`

Group: Speculation · Draft CPU placement. Aliases: `--spec-draft-cpu-strict-batch`, `--cpu-strict-batch-draft`.

Draft model only, not the target model: Prompt-processing phase: Control whether workers strictly follow the requested CPU placement.

- **Change:** Draft model only, not the target model: Prompt-processing phase: 1 enforces placement more strictly; 0 permits normal flexibility. Strict placement can help locality or hurt load balancing.
- **How much:** This is a policy switch, not a RAM or thread multiplier. Compare both on your actual multi-core/NUMA layout.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Draft prompt scheduling priority — `--spec-draft-prio-batch`

Group: Speculation · Draft scheduling. Aliases: `--spec-draft-prio-batch`, `--prio-batch-draft`.

Draft model only, not the target model: Prompt-processing phase: How strongly the operating system favors main-model CPU work.

- **Change:** Draft model only, not the target model: Prompt-processing phase: Higher priority favors this process under contention; lower priority makes it yield more readily to other applications.
- **How much:** The levels are discrete, not speed percentages. Realtime can make the desktop unresponsive; use normal unless measurements justify a change.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft prompt busy polling — `--spec-draft-poll-batch`

Group: Speculation · Draft scheduling. Aliases: `--spec-draft-poll-batch`, `--poll-batch-draft`.

Draft model only, not the target model: Prompt-processing phase: How actively workers spin while waiting for generation work.

- **Change:** Draft model only, not the target model: Prompt-processing phase: Increase for less waiting latency at the cost of CPU time/power; decrease to let idle workers sleep more.
- **How much:** The reviewed executable uses 0/1 for this waiting-policy switch. 1 enables polling and 0 disables it; neither means a specified percentage speedup.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft tensor placement — `--spec-draft-override-tensor`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-override-tensor`, `-otd`, `--override-tensor-draft`.

Draft model only, not the target model: Map named tensor patterns to a specific backend buffer type.

- **Change:** Draft model only, not the target model: Moves matching tensors toward a chosen device/CPU memory, overriding normal placement. Poor maps can add frequent transfers.
- **How much:** Memory moves by the actual matched tensor bytes, not by a fixed percentage. This is an expert setting; verify the placement in startup output.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft CPU MoE experts — `--spec-draft-cpu-moe`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-cpu-moe`, `-cmoed`, `--cpu-moe-draft`.

Draft model only, not the target model: Keep mixture-of-experts weights in system memory instead of putting them all on GPU.

- **Change:** Draft model only, not the target model: On can save substantial VRAM but makes expert evaluation/transfers depend on CPU/RAM bandwidth; off permits normal GPU placement.
- **How much:** Savings can be gigabytes for large MoE models, but expert sizes differ. This does not apply equivalently to dense FFN models.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft CPU MoE layer count — `--spec-draft-n-cpu-moe`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-n-cpu-moe`, `--spec-draft-ncmoe`, `-ncmoed`, `--n-cpu-moe-draft`.

Draft model only, not the target model: Keep expert weights for the first N MoE layers on CPU.

- **Change:** Draft model only, not the target model: Increase N to trade more GPU weight memory for CPU/RAM work; decrease to allow more GPU placement.
- **How much:** If five affected layers hold 200 MiB of experts each, their weight placement shifts by about 1 GiB. This is an illustration; use actual tensor sizes.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Maximum draft tokens — `--spec-draft-n-max`

Group: Speculation strategy. Aliases: `--spec-draft-n-max`.

Most tokens proposed in one speculative step.

- **Change:** Increase to potentially accept longer runs but spend more on rejected proposals; decrease to keep proposal overhead small.
- **How much:** 3 → 6 doubles the proposal ceiling, not generation speed. If typical acceptance is only 1–2 tokens, the extra proposals may be wasted work.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Minimum draft length — `--spec-draft-n-min`

Group: Speculation strategy. Aliases: `--spec-draft-n-min`.

Minimum proposed length used by the supported draft strategy.

- **Change:** Higher minima demand longer proposal work and can reduce flexibility; lower minima allow shorter drafts.
- **How much:** Keep it consistent with the maximum and selected method. The count controls proposals, not the final answer's maximum length.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Synthetic acceptance length — `--spec-synth-len`

Group: Speculation · synthetic benchmarks. Aliases: `--spec-synth-len`.

Force a synthetic mean acceptance length for benchmarking speculative mechanics.

- **Change:** Higher values simulate longer accepted runs; lower values simulate shorter runs. These are artificial acceptance conditions.
- **How much:** Includes the target token. Synthetic improvements are not evidence that a real draft model will reach that acceptance or speed on user prompts.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Synthetic acceptance probabilities — `--spec-synth-rates`

Group: Speculation · synthetic benchmarks. Aliases: `--spec-synth-rates`.

Artificial per-position acceptance probabilities for benchmarking.

- **Change:** Higher probabilities simulate more acceptance; lower ones simulate more rejected work.
- **How much:** A sequence of probability values is not a measured acceptance trace. Leave unset for real quality/speed evaluations.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Draft branching threshold — `--spec-draft-p-split`

Group: Speculation strategy. Aliases: `--spec-draft-p-split`, `--draft-p-split`.

Probability parameter used by supported speculative splitting/branching logic.

- **Change:** Changes which proposal branches are considered. Effect depends on strategy; it is not a universal higher-is-faster slider.
- **How much:** The reviewed command help names a split probability but does not establish a cross-strategy cost/speed formula. Keep its default unless tuning the particular implementation.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Draft confidence cutoff — `--spec-draft-p-min`

Group: Speculation strategy. Aliases: `--spec-draft-p-min`, `--draft-p-min`.

Minimum proposal probability used by supported greedy draft logic.

- **Change:** Increase to reject low-confidence proposals earlier, often shortening drafts; decrease to permit more uncertain proposals and possible rejection overhead.
- **How much:** 0.2 requires more draft confidence than 0.05; a draft probability is not the target model's acceptance rate or a guaranteed speedup.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Draft sampling on backend — `--spec-draft-backend-sampling`

Group: Speculation strategy. Aliases: `--spec-draft-backend-sampling`, `--no-spec-draft-backend-sampling`.

Perform supported draft-token selection on the device backend.

- **Change:** On may reduce draft sampling transfers/host work; off uses the ordinary alternative. Backend support determines whether it helps.
- **How much:** This is separate from target backend sampling. It cannot remove draft weight/KV memory or guarantee faster verification.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### How draft tokens are chosen — `--spec-draft-sampling`

Group: Speculation strategy. Aliases: `--spec-draft-sampling`.

Choose greedy proposals or supported probabilistic proposal/verification.

- **Change:** Greedy takes a most-likely candidate; probabilistic uses sampling with target verification. These are algorithms, not low/high quality levels.
- **How much:** Compatible target verification is essential. Compare acceptance and final sampling behavior with the installed version's chosen strategy.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Draft compute devices — `--spec-draft-device`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-device`, `-devd`, `--device-draft`.

Draft model only, not the target model: Choose the backend devices allowed to hold/run offloaded model work.

- **Change:** Draft model only, not the target model: Add devices to make multi-GPU placement possible; select none for CPU execution. A list by itself does not choose a balanced split.
- **How much:** Device names must come from this executable's list-devices output. More GPU memory can fit larger models, but communication can erase speed gains.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft GPU weight layers — `--spec-draft-ngl`

Group: Speculation · Draft cache & placement. Aliases: `--spec-draft-ngl`, `-ngld`, `--gpu-layers-draft`, `--n-gpu-layers-draft`.

Draft model only, not the target model: How many model layers' weights/work are offloaded to GPU. KV cache is a separate control.

- **Change:** Draft model only, not the target model: More generally moves weight work from CPU to GPU and consumes VRAM; fewer can fit smaller GPUs but leave more CPU work. GPU speed/interconnect and memory fit determine the gain.
- **How much:** Illustration only: 10 more layers averaging 100 MiB add about 1000 MiB (0.98 GiB) of GPU weights. Layers are unequal and extra tensors/buffers exist; 20 → 40 layers is not a guaranteed 2× speedup.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Draft model file — `--spec-draft-model`

Group: Speculation strategy. Aliases: `--spec-draft-model`, `-md`, `--model-draft`.

Smaller/compatible model used to propose tokens for the main model.

- **Change:** A faster draft can reduce proposal cost but may predict less accurately; a larger draft can improve acceptance while costing more compute/memory.
- **How much:** An extra 1 GiB draft requires its own weights plus draft KV/compute buffers; it is not free because the main model is already loaded.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Speculative decoding method — `--spec-type`

Group: Speculation strategy. Aliases: `--spec-type`.

Choose draft-model or history-based proposal methods for the target model to verify.

- **Change:** A good proposal method can emit multiple accepted tokens per target pass; bad proposals add rejected work. none disables speculation.
- **How much:** Draft methods can require additional weights; n-gram methods reuse patterns without a second model. Compare accepted tokens per step and actual tokens/s, not only proposal count.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram-mod minimum proposal — `--spec-ngram-mod-n-min`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-mod-n-min`.

Minimum draft run requested by the n-gram-mod history strategy.

- **Change:** A larger minimum requests longer candidate runs and can waste verification work if matches are weak; a smaller one accepts shorter opportunities.
- **How much:** 48 → 64 increases a proposal-length bound, not model context or a guaranteed acceptance count. Keep min/max consistent.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram-mod maximum proposal — `--spec-ngram-mod-n-max`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-mod-n-max`.

Maximum history-derived token run proposed by n-gram-mod.

- **Change:** Increase to exploit long repetitive matches; decrease to limit speculative verification work and rejected tokens.
- **How much:** 64 → 128 doubles the proposed-token ceiling, not generated tokens/s. Benefits depend on repetition in the actual request/history.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram-mod lookup length — `--spec-ngram-mod-n-match`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-mod-n-match`.

How many history tokens must match when finding a continuation.

- **Change:** Longer keys are more selective and may miss useful matches; shorter keys find more candidates that may be less reliable.
- **How much:** A 24-token key requires a longer repeated suffix than a 12-token key. This is not the model's context limit.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Simple n-gram lookup length — `--spec-ngram-simple-size-n`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-simple-size-n`.

Token-key length used to look up repeated continuations in ngram-simple.

- **Change:** Increase for stricter/rarer matches; decrease for more frequent but potentially weaker matches.
- **How much:** 12 → 24 doubles required key length, not speed or model memory. Use repetitive workloads to assess whether matches are useful.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Simple n-gram proposal length — `--spec-ngram-simple-size-m`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-simple-size-m`.

Length of candidate continuation ngram-simple may propose after a match.

- **Change:** Higher values allow longer candidate runs but can add rejected verification work; lower values cap that work.
- **How much:** 48 → 96 doubles the proposal-length ceiling, not accepted token count. No second draft-model weights are needed for this history method.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Simple n-gram evidence threshold — `--spec-ngram-simple-min-hits`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-simple-min-hits`.

How many matching observations are required before trusting a history proposal.

- **Change:** Higher thresholds require more repeated evidence and reduce opportunities; lower ones enable proposals sooner.
- **How much:** 1 → 3 requires three observations rather than one. This can improve confidence while leaving fewer opportunities to accelerate generation.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k lookup length — `--spec-ngram-map-k-size-n`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k-size-n`.

Applies to ngram-map-k, not ngram-simple: Token-key length used to look up repeated continuations in ngram-simple.

- **Change:** Applies to ngram-map-k, not ngram-simple: Increase for stricter/rarer matches; decrease for more frequent but potentially weaker matches.
- **How much:** 12 → 24 doubles required key length, not speed or model memory. Use repetitive workloads to assess whether matches are useful.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k proposal length — `--spec-ngram-map-k-size-m`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k-size-m`.

Applies to ngram-map-k, not ngram-simple: Length of candidate continuation ngram-simple may propose after a match.

- **Change:** Applies to ngram-map-k, not ngram-simple: Higher values allow longer candidate runs but can add rejected verification work; lower values cap that work.
- **How much:** 48 → 96 doubles the proposal-length ceiling, not accepted token count. No second draft-model weights are needed for this history method.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k evidence threshold — `--spec-ngram-map-k-min-hits`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k-min-hits`.

Applies to ngram-map-k, not ngram-simple: How many matching observations are required before trusting a history proposal.

- **Change:** Applies to ngram-map-k, not ngram-simple: Higher thresholds require more repeated evidence and reduce opportunities; lower ones enable proposals sooner.
- **How much:** 1 → 3 requires three observations rather than one. This can improve confidence while leaving fewer opportunities to accelerate generation.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k4v lookup length — `--spec-ngram-map-k4v-size-n`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k4v-size-n`.

Applies to ngram-map-k4v, not ngram-simple: Token-key length used to look up repeated continuations in ngram-simple.

- **Change:** Applies to ngram-map-k4v, not ngram-simple: Increase for stricter/rarer matches; decrease for more frequent but potentially weaker matches.
- **How much:** 12 → 24 doubles required key length, not speed or model memory. Use repetitive workloads to assess whether matches are useful.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k4v proposal length — `--spec-ngram-map-k4v-size-m`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k4v-size-m`.

Applies to ngram-map-k4v, not ngram-simple: Length of candidate continuation ngram-simple may propose after a match.

- **Change:** Applies to ngram-map-k4v, not ngram-simple: Higher values allow longer candidate runs but can add rejected verification work; lower values cap that work.
- **How much:** 48 → 96 doubles the proposal-length ceiling, not accepted token count. No second draft-model weights are needed for this history method.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### N-gram map-k4v evidence threshold — `--spec-ngram-map-k4v-min-hits`

Group: Speculation · n-gram history. Aliases: `--spec-ngram-map-k4v-min-hits`.

Applies to ngram-map-k4v, not ngram-simple: How many matching observations are required before trusting a history proposal.

- **Change:** Applies to ngram-map-k4v, not ngram-simple: Higher thresholds require more repeated evidence and reduce opportunities; lower ones enable proposals sooner.
- **How much:** 1 → 3 requires three observations rather than one. This can improve confidence while leaving fewer opportunities to accelerate generation.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Legacy maximum draft tokens — `--draft`

Group: Legacy speculation flags. Aliases: `--draft`, `--draft-n`, `--draft-max`.

Old name for a proposal-length control; removed in the reviewed executable.

- **Change:** Do not enable when the selected help says removed. Use spec-draft-n-max or the matching n-gram maximum instead.
- **How much:** Older executables may still implement this name. Follow that executable's help; the launcher preserves raw syntax/defaults alongside this guide.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Legacy minimum draft tokens — `--draft-min`

Group: Legacy speculation flags. Aliases: `--draft-min`, `--draft-n-min`.

Old minimum-draft control removed in the reviewed executable.

- **Change:** Use the selected method's supported minimum proposal flag instead of passing a removed argument.
- **How much:** This is not an output length limit. Read the installed help before using a legacy name from an older profile.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Legacy n-gram lookup length — `--spec-ngram-size-n`

Group: Legacy speculation flags. Aliases: `--spec-ngram-size-n`.

Old shared lookup-length flag replaced by method-specific options.

- **Change:** Use spec-ngram-simple/map-k/map-k4v-size-n or the relevant n-gram-mod match option supported by the executable.
- **How much:** The selected method determines the correct replacement; a common legacy value is not guaranteed meaningful for every new strategy.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Legacy n-gram proposal length — `--spec-ngram-size-m`

Group: Legacy speculation flags. Aliases: `--spec-ngram-size-m`.

Old shared proposal-length flag replaced by method-specific options.

- **Change:** Use the matching method's size-m/maximum proposal option where supported.
- **How much:** Do not add a removed flag to launch arguments. Older binaries can differ; preserve their original help for compatibility.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Legacy n-gram hit threshold — `--spec-ngram-min-hits`

Group: Legacy speculation flags. Aliases: `--spec-ngram-min-hits`.

Old shared minimum-observation flag replaced by method-specific thresholds.

- **Change:** Use the specific n-gram method's supported min-hits setting.
- **How much:** A removed flag can make startup fail rather than tune performance. Migrate it explicitly when changing runtime versions.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Static speculation lookup cache — `--lookup-cache-static`

Group: Speculation · n-gram history. Aliases: `-lcs`, `--lookup-cache-static`.

Read a prebuilt continuation lookup table that generation does not update.

- **Change:** A larger/more relevant table may find more useful proposals but requires storage/loading memory; an irrelevant one adds little benefit.
- **How much:** This is history/lookup data, not the active attention KV cache. A file twice as large does not imply twice the hit rate or speed.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Learning speculation lookup cache — `--lookup-cache-dynamic`

Group: Speculation · n-gram history. Aliases: `-lcd`, `--lookup-cache-dynamic`.

Use a lookup table that can be updated by generated text.

- **Change:** Can gain useful repeated continuations over time, with lookup/update and growing storage/memory costs.
- **How much:** Workload repetition determines benefit. Do not confuse it with cache-ram or the main model's quantized attention cache.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

### Automatic speculation configuration — `--spec-default`

Group: Speculation strategy. Aliases: `--spec-default`.

Request the runtime's recommended speculative setup where supported.

- **Change:** Can choose compatible speculation settings; disable/unset it for a normal baseline. The selected model and runtime determine what is available.
- **How much:** A preset is not a guaranteed speedup. Check the resolved method, draft model/memory and acceptance in server diagnostics.
- **Speed:** Extra proposal work helps only when enough tokens are accepted; otherwise speculation can slow generation.
- **VRAM:** A draft model and its KV/compute buffers can add VRAM; n-gram methods do not require a second weight model.
- **RAM:** Draft weights, KV/history and lookup tables can add RAM. Speed gains are acceptance/model dependent.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/examples/speculative/README.md).

## Settings → Performance

### CPU generation threads — `--threads`

Group: CPU threads. Aliases: `-t`, `--threads`.

How many CPU workers evaluate each newly generated token.

- **Change:** Increase to use more cores; decrease if scheduling or memory bandwidth becomes the bottleneck. GPU-heavy runs can need fewer CPU threads.
- **How much:** Try 2 → 4 → 8 and compare generated tokens/s. More workers can be slower; start near physical-core count rather than assuming all logical threads are best.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### CPU prompt-processing threads — `--threads-batch`

Group: CPU threads. Aliases: `-tb`, `--threads-batch`.

CPU workers used while reading the prompt in batches.

- **Change:** More workers may shorten prompt ingestion; fewer leave CPU capacity for other work. This is separate from generation threads.
- **How much:** Changing 4 → 8 doubles the requested worker count, not guaranteed prompt speed or RAM. Compare prompt tokens/s and time to first token.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### CPU affinity mask — `--cpu-mask`

Group: CPU placement & affinity. Aliases: `-C`, `--cpu-mask`.

Choose the exact CPU cores the main model may use, using a hexadecimal bit mask.

- **Change:** Adding bits permits more cores; removing bits restricts placement. A mask is not a thread-count setting.
- **How much:** For CPUs numbered 0–3, mask F permits those four cores. Choose according to your OS topology; a wider mask does not guarantee faster generation.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### CPU core range — `--cpu-range`

Group: CPU placement & affinity. Aliases: `-Cr`, `--cpu-range`.

Restrict the main model to a numbered range of CPU cores.

- **Change:** Widen the range to allow more cores; narrow it for isolation or locality. Also set a suitable thread count.
- **How much:** A range such as 0-3 allows four numbered CPUs; those may be logical siblings rather than four physical cores.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Strict CPU placement — `--cpu-strict`

Group: CPU placement & affinity. Aliases: `--cpu-strict`.

Control whether workers strictly follow the requested CPU placement.

- **Change:** 1 enforces placement more strictly; 0 permits normal flexibility. Strict placement can help locality or hurt load balancing.
- **How much:** This is a policy switch, not a RAM or thread multiplier. Compare both on your actual multi-core/NUMA layout.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### CPU scheduling priority — `--prio`

Group: Scheduling & polling. Aliases: `--prio`.

How strongly the operating system favors main-model CPU work.

- **Change:** Higher priority favors this process under contention; lower priority makes it yield more readily to other applications.
- **How much:** The levels are discrete, not speed percentages. Realtime can make the desktop unresponsive; use normal unless measurements justify a change.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Generation busy polling — `--poll`

Group: Scheduling & polling. Aliases: `--poll`.

How actively workers spin while waiting for generation work.

- **Change:** Increase for less waiting latency at the cost of CPU time/power; decrease to let idle workers sleep more.
- **How much:** 0 disables polling; 100 is the highest polling level, not 100% faster. Idle CPU use can change much more than tokens/s.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Prompt CPU affinity mask — `--cpu-mask-batch`

Group: CPU placement & affinity. Aliases: `-Cb`, `--cpu-mask-batch`.

Prompt-processing phase: Choose the exact CPU cores the main model may use, using a hexadecimal bit mask.

- **Change:** Prompt-processing phase: Adding bits permits more cores; removing bits restricts placement. A mask is not a thread-count setting.
- **How much:** For CPUs numbered 0–3, mask F permits those four cores. Choose according to your OS topology; a wider mask does not guarantee faster generation.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Prompt CPU core range — `--cpu-range-batch`

Group: CPU placement & affinity. Aliases: `-Crb`, `--cpu-range-batch`.

Prompt-processing phase: Restrict the main model to a numbered range of CPU cores.

- **Change:** Prompt-processing phase: Widen the range to allow more cores; narrow it for isolation or locality. Also set a suitable thread count.
- **How much:** A range such as 0-3 allows four numbered CPUs; those may be logical siblings rather than four physical cores.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Strict prompt CPU placement — `--cpu-strict-batch`

Group: CPU placement & affinity. Aliases: `--cpu-strict-batch`.

Prompt-processing phase: Control whether workers strictly follow the requested CPU placement.

- **Change:** Prompt-processing phase: 1 enforces placement more strictly; 0 permits normal flexibility. Strict placement can help locality or hurt load balancing.
- **How much:** This is a policy switch, not a RAM or thread multiplier. Compare both on your actual multi-core/NUMA layout.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Prompt scheduling priority — `--prio-batch`

Group: Scheduling & polling. Aliases: `--prio-batch`.

Prompt-processing phase: How strongly the operating system favors main-model CPU work.

- **Change:** Prompt-processing phase: Higher priority favors this process under contention; lower priority makes it yield more readily to other applications.
- **How much:** The levels are discrete, not speed percentages. Realtime can make the desktop unresponsive; use normal unless measurements justify a change.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Prompt busy polling — `--poll-batch`

Group: Scheduling & polling. Aliases: `--poll-batch`.

Prompt-processing phase: How actively workers spin while waiting for generation work.

- **Change:** Prompt-processing phase: Increase for less waiting latency at the cost of CPU time/power; decrease to let idle workers sleep more.
- **How much:** The reviewed executable uses 0/1 for this waiting-policy switch. 1 enables polling and 0 disables it; neither means a specified percentage speedup.
- **Speed:** Changes how eagerly CPU work is scheduled, not how much model arithmetic is required.
- **VRAM:** No direct change to model/KV VRAM.
- **RAM:** Usually little direct RAM change; CPU utilization, power and responsiveness matter more.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Logical prompt batch — `--batch-size`

Group: Batching & concurrency. Aliases: `-b`, `--batch-size`.

Maximum number of prompt tokens offered in one logical batch.

- **Change:** Increase to admit larger chunks and potentially better aggregate prompt throughput; decrease to limit work admitted at once. Actual execution is split by microbatch size.
- **How much:** 2048 → 4096 doubles the logical ceiling. With microbatch fixed at 512 it need not double peak compute-buffer memory or token speed.
- **Speed:** Mainly affects prompt ingestion and throughput; single-token generation is a different workload.
- **VRAM:** Larger physical batches can need larger compute buffers. Allocation is backend/model dependent.
- **RAM:** CPU compute buffers can grow with the physical batch; this does not change weight size.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Physical prompt microbatch — `--ubatch-size`

Group: Batching & concurrency. Aliases: `-ub`, `--ubatch-size`.

Maximum prompt tokens actually processed together in one physical computation.

- **Change:** Increase for more parallel prompt computation and potentially larger temporary buffers; decrease to save working memory at the cost of more passes.
- **How much:** 256 → 512 doubles the microbatch token ceiling. Temporary memory may grow, but its exact factor is backend/model dependent. Keep it at or below the logical batch.
- **Speed:** Mainly affects prompt ingestion and throughput; single-token generation is a different workload.
- **VRAM:** Larger physical batches can need larger compute buffers. Allocation is backend/model dependent.
- **RAM:** CPU compute buffers can grow with the physical batch; this does not change weight size.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### NUMA memory policy — `--numa`

Group: CPU placement & affinity. Aliases: `--numa`.

Choose how CPU workers and memory behave across NUMA nodes/sockets.

- **Change:** Distribute spreads work; isolate keeps work on the starting node; numactl follows the external CPU map. No mode is always fastest.
- **How much:** Most single-socket consumer PCs gain little. Cross-socket remote memory traffic can dominate; measure on the intended NUMA machine.
- **Speed:** CPU work may improve until memory bandwidth is saturated; doubling threads does not guarantee double speed.
- **VRAM:** No direct change to GPU weight or KV allocation.
- **RAM:** Extra thread stacks/work buffers are small relative to weights; model size stays the same.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Warm up the model — `--warmup`

Group: Startup. Aliases: `--warmup`, `--no-warmup`.

Run a small initial evaluation before accepting normal work.

- **Change:** On adds startup work but can reduce first-request initialization delay; off reaches startup sooner but shifts that cost to an early request.
- **How much:** Changes startup/first-request latency rather than a fixed steady-state speed percentage. Measure both separately.
- **Speed:** Primarily changes startup, disk access and paging; it is not a fixed tokens/second multiplier.
- **VRAM:** For unchanged offload/model settings, weight VRAM usually stays similar.
- **RAM:** Mapped address space is not resident RAM. Locked pages cannot be paged out; lazy reads trade RAM for disk access.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Concurrent request slots — `--parallel`

Group: Batching & concurrency. Aliases: `-np`, `--parallel`.

How many requests the server may work on at once.

- **Change:** More slots can improve total service throughput but compete for compute and context capacity; fewer favor individual request latency.
- **How much:** 1 → 4 means up to four slots, not four weight copies or four times the speed. With a fixed total context, per-slot room can fall; per-slot context controls can instead grow the pool.
- **Speed:** Mainly affects prompt ingestion and throughput; single-token generation is a different workload.
- **VRAM:** Larger physical batches can need larger compute buffers. Allocation is backend/model dependent.
- **RAM:** CPU compute buffers can grow with the physical batch; this does not change weight size.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Continuous request batching — `--cont-batching`

Group: Batching & concurrency. Aliases: `-cb`, `--cont-batching`, `-nocb`, `--no-cont-batching`.

Let active/new requests share inference batches as slots become available.

- **Change:** On can improve multi-user utilization; off removes this dynamic batching behavior and may simplify comparison of isolated requests.
- **How much:** Most relevant with multiple simultaneous requests. A single chat is not guaranteed to become faster just by enabling it.
- **Speed:** Mainly affects prompt ingestion and throughput; single-token generation is a different workload.
- **VRAM:** Larger physical batches can need larger compute buffers. Allocation is backend/model dependent.
- **RAM:** CPU compute buffers can grow with the physical batch; this does not change weight size.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Settings → Context & cache

### Context length in tokens — `--ctx-size`

Group: Context capacity. Aliases: `-c`, `--ctx-size`.

Total token context available for prompt history and generated answers.

- **Change:** More context fits longer conversations/documents but uses more KV capacity and can make attention slower; less saves cache memory and limits retained text.
- **How much:** For the same dense-attention model/cache layout, 4096 → 8192 is approximately 2× KV capacity, not 2× total memory. Example: a 1 GiB KV cache becomes about 2 GiB; weights stay unchanged. 0 follows the model.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Initial prompt tokens to retain — `--keep`

Group: Context capacity. Aliases: `--keep`.

How much of the initial prompt survives when old context must be shifted.

- **Change:** More retained tokens protect the beginning/system prompt but leave less room for newer history; fewer let more of the original text be discarded.
- **How much:** Keeping 512 of a 4096-token window reserves roughly one eighth for initial tokens during shifting. It does not allocate an extra 512-token cache; -1 requests all initial tokens.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Full sliding-window cache — `--swa-full`

Group: Context capacity. Aliases: `--swa-full`.

Use a full-size cache rather than the optimized smaller sliding-window arrangement where supported.

- **Change:** On retains a larger cache and can use much more memory; off permits the sliding-window optimization. Only models using that attention layout are affected.
- **How much:** There is no universal ratio: it depends on the attention window, layer mix and requested context. Do not apply the dense-cache estimate blindly to hybrid/SWA models.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### RoPE extension method — `--rope-scaling`

Group: Extended context (model-specific). Aliases: `--rope-scaling`.

Choose the positional-encoding adjustment used for context extension.

- **Change:** none preserves unscaled positions; linear and yarn use different extension strategies. Changing it alone does not enlarge the allocated context.
- **How much:** Use the model's documented method. Extending beyond training context can reduce quality even when the prompt fits in memory.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### RoPE context scale — `--rope-scale`

Group: Extended context (model-specific). Aliases: `--rope-scale`.

Model-specific factor used to extend the positional range.

- **Change:** A larger factor stretches the positional range further; a smaller factor stays closer to the original range, with model-dependent quality effects.
- **How much:** A factor of 2 requests roughly a 2× positional range, not 2× speed. Raising ctx-size separately raises actual cache allocation.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### RoPE base frequency — `--rope-freq-base`

Group: Extended context (model-specific). Aliases: `--rope-freq-base`.

Base frequency used by supported positional-scaling methods.

- **Change:** Changing this alters positional behavior, not a simple speed/memory slider. Wrong values can harm long-context answers.
- **How much:** Use the model's metadata or recommended value. There is no generally correct higher/lower setting or universal performance gain.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### RoPE inverse frequency scale — `--rope-freq-scale`

Group: Extended context (model-specific). Aliases: `--rope-freq-scale`.

Frequency scale whose inverse controls the positional stretch.

- **Change:** Smaller positive values stretch farther; larger values stretch less. This is the inverse direction of rope-scale.
- **How much:** 0.5 corresponds to a 2× positional-range factor; 0.25 to 4×. Cache size still comes from context settings, and answer quality is not guaranteed.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Original training context for YaRN — `--yarn-orig-ctx`

Group: Extended context (model-specific). Aliases: `--yarn-orig-ctx`.

Reference training context used by YaRN scaling.

- **Change:** This should describe the model, not be increased to request a larger prompt. A wrong reference changes the scaling calculation.
- **How much:** 0 reads the model training-context value. Set an explicit number only when the model's instructions require it.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### YaRN extrapolation mix — `--yarn-ext-factor`

Group: Extended context (model-specific). Aliases: `--yarn-ext-factor`.

Blend interpolation and extrapolation when applying YaRN.

- **Change:** Increasing changes the mix toward extrapolation; 0 uses full interpolation. This is a positional-quality control, not a memory budget.
- **How much:** The reviewed runtime uses -1 for automatic behavior. Use model-specific guidance; no universal percentage improvement exists.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### YaRN attention magnitude — `--yarn-attn-factor`

Group: Extended context (model-specific). Aliases: `--yarn-attn-factor`.

Scale the attention magnitude used with YaRN extension.

- **Change:** Higher/lower values alter attention weighting and can affect output quality; neither direction guarantees faster inference.
- **How much:** Keep the model/automatic value unless its extension recipe specifies otherwise. It does not directly allocate more KV capacity.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### YaRN slow correction bound — `--yarn-beta-slow`

Group: Extended context (model-specific). Aliases: `--yarn-beta-slow`.

One bound controlling which positional dimensions receive YaRN correction.

- **Change:** Changes the correction distribution; it is not a token-count, thread or memory multiplier.
- **How much:** Tune together with beta-fast using the model's documented recipe. Leave automatic/model values for ordinary use.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### YaRN fast correction bound — `--yarn-beta-fast`

Group: Extended context (model-specific). Aliases: `--yarn-beta-fast`.

The other bound controlling YaRN's positional correction distribution.

- **Change:** Changes positional correction relative to beta-slow; unsuitable bounds can reduce long-context quality.
- **How much:** There is no safe universal higher-is-better rule. It does not independently enlarge the model's trained context or cache.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Put KV cache on GPU — `--kv-offload`

Group: KV cache precision & placement. Aliases: `-kvo`, `--kv-offload`, `-nkvo`, `--no-kv-offload`.

Control GPU placement of attention's stored keys/values; this is not the GPU layer-count setting.

- **Change:** On uses GPU memory for eligible KV storage and can reduce CPU transfers; off shifts eligible cache toward system RAM and can slow attention.
- **How much:** If 2 GiB of eligible KV storage moves off the GPU, VRAM may fall by about that cache amount and RAM rise correspondingly. Weight placement is unchanged; actual layer/backend eligibility matters.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Key cache precision — `--cache-type-k`

Group: KV cache precision & placement. Aliases: `-ctk`, `--cache-type-k`.

Storage format for the attention key cache, separate from model-weight quantization.

- **Change:** Lower precision reduces this cache's memory/bandwidth but can reduce quality or add conversion work; higher precision uses more memory. Backend/model support varies.
- **How much:** Raw-storage estimate: f16 → q8_0 uses about 53%; q4_0 about 28%. A 2 GiB K payload becomes roughly 1.06 GiB or 0.56 GiB, before alignment/metadata. This changes K only, not total application memory.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Value cache precision — `--cache-type-v`

Group: KV cache precision & placement. Aliases: `-ctv`, `--cache-type-v`.

Storage format for the attention value cache, separate from the key cache.

- **Change:** Lower precision saves V-cache memory with possible quality/conversion costs; higher precision retains more detail and costs more memory. Quantized V can require compatible Flash Attention/backends.
- **How much:** If equal-sized K and V both change f16 → q8_0, a 2 GiB dense KV payload becomes roughly 1.06 GiB (53%); changing only V leaves about 1.53 GiB (77% combined), before overhead. q4_0 is about 28% per changed component; quality/speed are not guaranteed.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Legacy KV defragmentation threshold — `--defrag-thold`

Group: KV cache precision & placement. Aliases: `-dt`, `--defrag-thold`.

A legacy threshold for reorganizing fragmented KV storage.

- **Change:** This option is deprecated in the reviewed executable; changing it may be ignored. Check the selected executable's original help before using it.
- **How much:** No reliable modern tuning magnitude exists. Leave it unset and use supported context/cache controls instead.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/ggml/src/ggml-common.h).

### Model loading mode — `--load-mode`

Group: Model loading & paging. Aliases: `-lm`, `--load-mode`.

Choose mapped, locked, ordinary or DirectIO weight loading.

- **Change:** mmap loads pages as needed; mlock prevents eligible pages from being swapped; mmap+mlock combines them; dio changes disk I/O. Modes are not ordered from slow to fast.
- **How much:** Locking 8 GiB of eligible pages can require roughly 8 GiB of available resident RAM, plus cache/buffers. It does not reduce GPU weight bytes or guarantee more tokens/s.
- **Speed:** Primarily changes startup, disk access and paging; it is not a fixed tokens/second multiplier.
- **VRAM:** For unchanged offload/model settings, weight VRAM usually stays similar.
- **RAM:** Mapped address space is not resident RAM. Locked pages cannot be paged out; lazy reads trade RAM for disk access.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Read large tensors on demand — `--lazy-mode`

Group: Model loading & paging. Aliases: `-lzm`, `--lazy-mode`.

Allow supported tensor rows to be read from disk instead of remaining resident.

- **Change:** on can reduce resident RAM but add read latency; off keeps supported tensors resident; auto only applies above the runtime's size threshold.
- **How much:** The reviewed auto threshold is tensors larger than 4 GiB. Fast SSDs and access patterns matter; this is not a generic guarantee that all model weights become lazy.
- **Speed:** Primarily changes startup, disk access and paging; it is not a fixed tokens/second multiplier.
- **VRAM:** For unchanged offload/model settings, weight VRAM usually stays similar.
- **RAM:** Mapped address space is not resident RAM. Locked pages cannot be paged out; lazy reads trade RAM for disk access.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Context limit per slot — `--kv-unified-per-slot`

Group: Context capacity. Aliases: `--kv-unified-per-slot`.

Set a token limit for each parallel request when using the supported shared KV pool.

- **Change:** A larger per-slot limit permits longer requests and can size a larger total pool; a smaller one saves capacity when the pool is derived from slot count.
- **How much:** With no explicit total context, 4 slots × 4096 tokens requires a 16384-token pool; 4 × 8192 requires 32768. An explicit context value and runtime version can change that interaction.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Context checkpoints per slot — `--ctx-checkpoints`

Group: Prompt reuse & checkpoints. Aliases: `-ctxcp`, `--ctx-checkpoints`, `--swa-checkpoints`.

Maximum intermediate context snapshots retained per request slot.

- **Change:** More checkpoints can help restore/reuse earlier state but add memory; fewer save snapshot memory and can increase recomputation.
- **How much:** 32 → 64 doubles the checkpoint-count ceiling, not necessarily total RAM. Recurrent/hybrid state size and active slot count determine the cost.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Checkpoint spacing in tokens — `--checkpoint-min-step`

Group: Prompt reuse & checkpoints. Aliases: `-cms`, `--checkpoint-min-step`.

Minimum token distance between stored context checkpoints.

- **Change:** Increase to create snapshots less often and save overhead; decrease to retain finer recovery points. 0 removes the minimum spacing.
- **How much:** 8192 → 4096 permits snapshots twice as often on a long sequence, up to the checkpoint-count limit. It does not halve generation time.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Host prompt-cache budget — `--cache-ram`

Group: Prompt reuse & checkpoints. Aliases: `-cram`, `--cache-ram`.

Maximum system-memory budget for saved prompt/state cache, in MiB.

- **Change:** Increase to retain more reusable states; decrease to limit host cache growth. 0 disables this cache and -1 removes the configured cap.
- **How much:** 8192 MiB permits about 8 GiB of cache budget; 4096 permits about 4 GiB. This is a cache limit, not immediate allocation or a cap on total process RAM.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Shared KV pool — `--kv-unified`

Group: Context capacity. Aliases: `-kvu`, `--kv-unified`, `-no-kvu`, `--no-kv-unified`.

Use a KV buffer that multiple request sequences can share.

- **Change:** On pools storage so slots can share available capacity; off uses the runtime's separate-sequence layout. This does not duplicate or quantize weights.
- **How much:** Pooling changes allocation/utilization, not a guaranteed memory percentage. Total context, slot count, per-slot limits and architecture still determine actual allocation.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Cache idle request slots — `--cache-idle-slots`

Group: Prompt reuse & checkpoints. Aliases: `--cache-idle-slots`, `--no-cache-idle-slots`.

Save idle slot state in the prompt cache when new work needs the slot.

- **Change:** On can preserve reusable context while freeing a shared KV slot; off can require more later recomputation. Needs the host prompt cache.
- **How much:** Savings depend on matching future requests and the cache-ram budget. It does not increase the number of simultaneously executing slots.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Shift a full context window — `--context-shift`

Group: Context capacity. Aliases: `--context-shift`, `--no-context-shift`.

Allow old context to be dropped/rearranged to continue generating when the window fills.

- **Change:** On permits continuation but the model loses discarded history and shifting may cost time; off can stop at the configured limit.
- **How much:** A 4096-token cache remains bounded around its configured capacity. This does not provide unlimited memory or unlimited recall.
- **Speed:** Longer retained attention history usually means more work per token; cache placement/precision also affects bandwidth.
- **VRAM:** GPU-resident KV storage changes; the model weights do not. Size depends on context, layers and KV-head dimensions.
- **RAM:** The same cache uses RAM when kept on CPU. Recurrent/hybrid/SWA models do not always follow dense-cache formulas.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Reuse matching prompts — `--cache-prompt`

Group: Prompt reuse & checkpoints. Aliases: `--cache-prompt`, `--no-cache-prompt`.

Reuse already evaluated matching prompt tokens instead of computing them again.

- **Change:** On can shorten repeated-prefix prompt processing; off forces more recomputation and is useful for uncached benchmark comparisons.
- **How much:** A repeated 3000-token prefix in a 3200-token request may avoid much of those 3000 tokens' evaluation, not all generation work. Gains require an actual reusable match.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Minimum reusable chunk — `--cache-reuse`

Group: Prompt reuse & checkpoints. Aliases: `--cache-reuse`.

Minimum token chunk worth trying to recover through KV shifting.

- **Change:** A lower positive threshold considers smaller reused chunks and can add lookup/shifting work; a higher one requires larger matches. 0 disables this reuse mode.
- **How much:** A threshold of 128 considers shorter matches than 512. Neither value guarantees a specific hit rate or speedup; prompt caching must be enabled.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Saved slot-cache directory — `--slot-save-path`

Group: Prompt reuse & checkpoints. Aliases: `--slot-save-path`.

Directory used to save and restore slot KV state on disk.

- **Change:** Enabling a path supports persistence/reuse but adds disk I/O and storage; leaving it unset disables that path-based feature.
- **How much:** Files can be large and can contain prompt-derived state. Disk usage depends on saved context/cache format; this is separate from the RAM cache budget.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

### Prompt similarity for slot reuse — `--slot-prompt-similarity`

Group: Prompt reuse & checkpoints. Aliases: `-sps`, `--slot-prompt-similarity`.

How much a request must resemble a slot's prompt before that slot is reused.

- **Change:** Higher thresholds demand closer matches and may reduce reuse; lower thresholds accept looser candidates. 0 disables this selection criterion.
- **How much:** 0.10 is a similarity threshold, not a 10% speedup or 10% memory allocation. Compare cache hits/time to first token for your prompts.
- **Speed:** A reusable matching prefix can avoid prompt recomputation; a new unrelated prompt gets little benefit.
- **VRAM:** Host prompt-cache limits do not directly set the active GPU KV allocation.
- **RAM:** This controls cached prompt/state storage in system memory; larger limits permit more RAM use.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp).

## Settings → Chat & output

### Output token limit — `--predict`

Group: Response length & stopping. Aliases: `-n`, `--predict`, `--n-predict`.

Maximum new tokens an answer may generate, separate from prompt/context length.

- **Change:** Increase to allow longer answers and longer total generation time; decrease to cap latency/output. It does not directly increase tokens/s.
- **How much:** At a measured 20 tokens/s, 200 generated tokens take roughly 10 s and 400 roughly 20 s, excluding prompt work. A maximum is not a promise to generate that many; negative sentinels are version-specific.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Interpret escape sequences — `--escape`

Group: Response length & stopping. Aliases: `-e`, `--escape`, `--no-escape`.

Interpret text escapes such as backslash-n as newline characters where supported.

- **Change:** On converts recognized escapes; off treats the relevant input literally.
- **How much:** The two characters backslash+n become one newline under escaping. Prompt tokenization may change; it is not a memory optimization.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Ignore the normal end token — `--ignore-eos`

Group: Response length & stopping. Aliases: `--ignore-eos`.

Prevent the model's end-of-sequence token from ending generation normally.

- **Change:** On can keep output going after a natural stop and consume the remaining limit/context; off respects the stop token.
- **How much:** A model that normally stops after 50 tokens may continue toward a 500-token limit. That increases elapsed work, not weight size.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Inline output grammar — `--grammar`

Group: Constrained output. Aliases: `--grammar`.

Restrict generated token sequences to a supplied grammar.

- **Change:** Stricter grammars narrow allowed output and can add token-filtering work; removing the constraint permits freer text.
- **How much:** Use for structural validity, not factual correctness. Large/ambiguous grammars can have materially higher filtering overhead.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Output grammar file — `--grammar-file`

Group: Constrained output. Aliases: `--grammar-file`.

Read a grammar that constrains the response from a file.

- **Change:** Same constraint tradeoff as inline grammar; file-based input is easier to maintain for complex rules.
- **How much:** A valid format does not guarantee a correct answer. Grammar compilation/filtering costs depend on its complexity, not just file length.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Inline JSON output schema — `--json-schema`

Group: Constrained output. Aliases: `-j`, `--json-schema`.

Constrain generated JSON to the supported subset of a schema.

- **Change:** More constraints restrict possible output; they may add filtering/setup work. Fewer constraints permit more JSON shapes.
- **How much:** A schema requiring one field does not make the field's value truthful. Not every JSON Schema construct is supported.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### JSON schema file — `--json-schema-file`

Group: Constrained output. Aliases: `-jf`, `--json-schema-file`.

Read the output schema from a file instead of inline JSON.

- **Change:** Changing schemas alters permitted output, not GPU weight placement.
- **How much:** Use the selected runtime's supported schema subset. Complex schemas can cost more setup/filtering time; no constant speed ratio applies.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Stop text sequence — `--reverse-prompt`

Group: Response length & stopping. Aliases: `-r`, `--reverse-prompt`.

Stop generation when the specified text appears.

- **Change:** A more likely/shorter stop string can end answers earlier; a rare one may never appear. This is a stop condition, not a decoding speed setting.
- **How much:** Choose text that cannot accidentally occur in a desired answer. It can truncate valid prose/code if the marker is too broad.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Show special tokens — `--special`

Group: Response length & stopping. Aliases: `-sp`, `--special`.

Include visible representations of model control/special tokens in output.

- **Change:** On exposes formatting/control tokens for inspection; off produces cleaner user-facing text.
- **How much:** Usually a small output-format change. It does not add training knowledge or enlarge the context limit.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Code infill ordering — `--spm-infill`

Group: Chat templates & infill. Aliases: `--spm-infill`.

Use suffix-prefix-middle ordering instead of prefix-suffix-middle for supported code models.

- **Change:** This is a format choice to match the model, not a larger/smaller setting.
- **How much:** Wrong infill ordering can make code completion ineffective. Use the order the chosen infill model expects.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Template parameters (JSON) — `--chat-template-kwargs`

Group: Chat templates & infill. Aliases: `--chat-template-kwargs`.

Pass additional named values to the chat template, such as a supported thinking switch.

- **Change:** Effect depends entirely on which keys the template reads. More keys do not inherently mean more sampling or more compute.
- **How much:** For a supporting template, enable_thinking can change output length substantially; an unsupported key may have no effect.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Jinja chat formatting — `--jinja`

Group: Chat templates & infill. Aliases: `--jinja`, `--no-jinja`.

Use the template engine that formats structured chat messages for the model.

- **Change:** On supports the model's Jinja chat template and tool formatting; off uses the supported alternative behavior.
- **How much:** Keep the model-compatible format. A wrong template can damage answers even while speed and memory look normal.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### How reasoning is returned — `--reasoning-format`

Group: Reasoning / thinking. Aliases: `--reasoning-format`.

Choose whether thought markers remain in content or are extracted into structured fields.

- **Change:** Changes visibility/response structure; hiding thought text does not necessarily prevent the model from generating it.
- **How much:** A shorter visible answer can still have spent time on hidden reasoning. Measure generated reasoning tokens/latency separately.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Thinking mode — `--reasoning`

Group: Reasoning / thinking. Aliases: `-rea`, `--reasoning`.

Enable, disable or automatically detect model/template-supported thinking.

- **Change:** On can add reasoning tokens/time; off requests non-thinking behavior where supported; auto follows the template/runtime.
- **How much:** This does not grant reasoning support to an incompatible model. Compare total generated tokens and wall time, not only tokens/s.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Thinking effort hint — `--reasoning-effort`

Group: Reasoning / thinking. Aliases: `--reasoning-effort`.

Pass a named effort level to a supporting chat template.

- **Change:** Higher levels can lead to longer/more deliberate thinking; lower levels can reduce response time. Exact behavior is model-defined.
- **How much:** high is not a fixed token multiplier and may be ignored by unsupported templates. Use a token budget when a supported hard limit is needed.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Thinking token budget — `--reasoning-budget`

Group: Reasoning / thinking. Aliases: `--reasoning-budget`.

Limit how many reasoning tokens are allowed before the runtime ends thinking.

- **Change:** Larger budgets permit more thinking time; smaller budgets end thinking earlier. -1 is unrestricted and 0 requests immediate end in the reviewed build.
- **How much:** At a measured 20 generated tokens/s, 1000 thinking tokens alone are roughly 50 s; 2000 roughly 100 s. Actual use can be below the cap and answer tokens add time.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Budget-exhausted message — `--reasoning-budget-message`

Group: Reasoning / thinking. Aliases: `--reasoning-budget-message`.

Text injected before closing the thinking section when its budget runs out.

- **Change:** Different messages can affect the transition into the answer; more text consumes additional prompt/state tokens.
- **How much:** This message is not the budget itself. Keep it short and compatible with the chosen model/template.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Keep earlier reasoning in history — `--reasoning-preserve`

Group: Reasoning / thinking. Aliases: `--reasoning-preserve`, `--no-reasoning-preserve`.

Retain prior reasoning traces across chat history where the template supports it.

- **Change:** On can grow future prompts/context use; off can reduce repeated reasoning history and change later answers.
- **How much:** Keeping 1000 additional reasoning tokens uses roughly 1000 more prompt positions next time; it does not duplicate model weights.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Inline chat template — `--chat-template`

Group: Chat templates & infill. Aliases: `--chat-template`.

Override how roles/messages are converted into the model's prompt.

- **Change:** Changing the template changes control tokens and prompt layout; it is unrelated to temperature sampling.
- **How much:** Prefer the GGUF's built-in template unless the model publisher provides another. More template text can consume more prompt tokens.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Chat template file — `--chat-template-file`

Group: Chat templates & infill. Aliases: `--chat-template-file`.

Read a custom Jinja chat template from a file.

- **Change:** Same formatting effect as an inline template; changing files does not change model weights.
- **How much:** Use a template intended for this exact model family. File size is not a model performance score.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Return raw content parsing — `--skip-chat-parsing`

Group: Chat templates & infill. Aliases: `--skip-chat-parsing`, `--no-skip-chat-parsing`.

Bypass the usual structured chat parser and place generated material in content.

- **Change:** On can leave reasoning/tool markers inside ordinary content; off permits supported structured extraction.
- **How much:** This changes response representation, not the number of model parameters. Clients expecting tool_calls or separated reasoning may need different handling.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Continue an assistant prefix — `--prefill-assistant`

Group: Chat templates & infill. Aliases: `--prefill-assistant`, `--no-prefill-assistant`.

Continue the final assistant message instead of always starting a fresh response.

- **Change:** On can constrain the beginning of an answer to a supplied prefix; off follows the non-prefill chat behavior.
- **How much:** Useful for compatible structured starts, but model/template support matters. The prefix consumes prompt tokens.
- **Speed:** Templates/constraints can change token counts or add parsing work. They do not make model inference intrinsically faster.
- **VRAM:** Weights are unchanged; longer prompts/outputs consume more of the configured context.
- **RAM:** Text, parser and constraint buffers can grow; usually small compared with weights, except very complex constraints.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Settings → GPU & memory

### Flash Attention — `--flash-attn`

Group: Attention & automatic fitting. Aliases: `-fa`, `--flash-attn`.

Use a fused attention implementation when supported by the backend/model.

- **Change:** On can reduce attention temporary buffers and improve long-context processing; off uses the alternative path; auto lets the runtime decide.
- **How much:** It does not shrink model weights or change the mathematical KV element count. Gains and supported cache formats depend on GPU/backend; no fixed speed percentage applies.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Repack weights for the backend — `--repack`

Group: Backend buffers & operations. Aliases: `--repack`, `-nr`, `--no-repack`.

Allow a backend to rearrange weights into a format better suited to its kernels.

- **Change:** On can add startup work/temporary storage but help supported kernels; off avoids that step and may lower their throughput.
- **How much:** It does not change GGUF quantization or parameter count. Memory/speed differences depend on the backend's repacking implementation.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Bypass host staging buffer — `--no-host`

Group: Backend buffers & operations. Aliases: `--no-host`.

Change backend buffer allocation by bypassing the usual host buffer.

- **Change:** Enable only for a backend/layout that benefits from the extra-buffer path; disabling returns to ordinary host-buffer behavior.
- **How much:** This is a memory/backend option, unrelated to listen address or network access. No fixed RAM/VRAM saving can be promised.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Remote compute devices — `--rpc`

Group: Backend buffers & operations. Aliases: `--rpc`.

Connect supported RPC backends to remote device servers.

- **Change:** Adding remote devices can increase available memory/compute but introduces network transfer and remote resource costs.
- **How much:** Network bandwidth/latency can dominate. This is remote inference computation, not the local API listen address or an MCP connection.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### GPU devices to use — `--device`

Group: Weight placement. Aliases: `-dev`, `--device`.

Choose the backend devices allowed to hold/run offloaded model work.

- **Change:** Add devices to make multi-GPU placement possible; select none for CPU execution. A list by itself does not choose a balanced split.
- **How much:** Device names must come from this executable's list-devices output. More GPU memory can fit larger models, but communication can erase speed gains.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Per-tensor device placement — `--override-tensor`

Group: Weight placement. Aliases: `-ot`, `--override-tensor`.

Map named tensor patterns to a specific backend buffer type.

- **Change:** Moves matching tensors toward a chosen device/CPU memory, overriding normal placement. Poor maps can add frequent transfers.
- **How much:** Memory moves by the actual matched tensor bytes, not by a fixed percentage. This is an expert setting; verify the placement in startup output.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Keep all MoE expert weights on CPU — `--cpu-moe`

Group: Weight placement. Aliases: `-cmoe`, `--cpu-moe`.

Keep mixture-of-experts weights in system memory instead of putting them all on GPU.

- **Change:** On can save substantial VRAM but makes expert evaluation/transfers depend on CPU/RAM bandwidth; off permits normal GPU placement.
- **How much:** Savings can be gigabytes for large MoE models, but expert sizes differ. This does not apply equivalently to dense FFN models.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### MoE layers with CPU experts — `--n-cpu-moe`

Group: Weight placement. Aliases: `-ncmoe`, `--n-cpu-moe`.

Keep expert weights for the first N MoE layers on CPU.

- **Change:** Increase N to trade more GPU weight memory for CPU/RAM work; decrease to allow more GPU placement.
- **How much:** If five affected layers hold 200 MiB of experts each, their weight placement shifts by about 1 GiB. This is an illustration; use actual tensor sizes.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### GPU cache for CPU experts — `--moe-cache-mib`

Group: Weight placement. Aliases: `--moe-cache-mib`.

Reserve a GPU cache for MoE experts otherwise kept on CPU.

- **Change:** Increase to retain more expert data on GPU and potentially reduce transfers; decrease to leave more VRAM for other buffers. 0 disables it.
- **How much:** 1024 → 2048 requests an extra 1024 MiB (1 GiB) cache budget across the configured devices. Hit rate determines whether that memory buys useful speed.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Dense FFN layers on CPU — `--n-cpu-ffn`

Group: Weight placement. Aliases: `-ncffn`, `--n-cpu-ffn`.

Keep feed-forward weights for the first N dense-model layers on CPU.

- **Change:** More CPU FFN layers can reduce VRAM and slow GPU-centric inference; fewer keep more FFN work on GPU. MoE experts use their separate setting.
- **How much:** Memory changes by the FFN tensors in the selected layers, not entire layers or the model's parameter count alone.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### GPU weight layers — `--gpu-layers`

Group: Weight placement. Aliases: `-ngl`, `--gpu-layers`, `--n-gpu-layers`.

How many model layers' weights/work are offloaded to GPU. KV cache is a separate control.

- **Change:** More generally moves weight work from CPU to GPU and consumes VRAM; fewer can fit smaller GPUs but leave more CPU work. GPU speed/interconnect and memory fit determine the gain.
- **How much:** Illustration only: 10 more layers averaging 100 MiB add about 1000 MiB (0.98 GiB) of GPU weights. Layers are unequal and extra tensors/buffers exist; 20 → 40 layers is not a guaranteed 2× speedup.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### How multiple GPUs share work — `--split-mode`

Group: Multi-GPU allocation. Aliases: `-sm`, `--split-mode`.

Choose a single GPU, layer split, row split or experimental tensor split.

- **Change:** Layer/row/tensor schemes distribute computation differently. More parallelism also introduces communication and mode-specific buffers.
- **How much:** Layer splitting is not equivalent to adding all GPUs' bandwidth. Benchmark the actual PCIe/NVLink topology; tensor mode is experimental in the reviewed build.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### GPU memory proportions — `--tensor-split`

Group: Multi-GPU allocation. Aliases: `-ts`, `--tensor-split`.

Relative proportions of model placement across the selected GPUs.

- **Change:** Increase one GPU's share to move more eligible tensors there; reduce it when that GPU needs headroom. Values are ratios, not GiB.
- **How much:** 3,1 targets about 75%/25% of the distributed portion, not 3 GiB/1 GiB. Some tensors, KV and intermediate buffers have separate placement rules.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Primary GPU index — `--main-gpu`

Group: Multi-GPU allocation. Aliases: `-mg`, `--main-gpu`.

Choose the single GPU or the device for shared/intermediate work in supported split modes.

- **Change:** Changing the index can move a concentration of work/memory. It does not add GPUs or increase their memory.
- **How much:** Index 0 → 1 selects a different device, not higher performance. Its exact effect depends on split mode; some modes ignore it.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Fit unset settings to device memory — `--fit`

Group: Attention & automatic fitting. Aliases: `-fit`, `--fit`.

Let llama.cpp adjust otherwise-unset context/offload choices to fit available device memory.

- **Change:** On can avoid out-of-memory startup by lowering/changing chosen settings; off leaves those automatic fitting adjustments disabled.
- **How much:** This does not override every explicit setting or guarantee all workloads fit. Read the final context/offload allocation in startup output.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Free VRAM margin per device — `--fit-target`

Group: Attention & automatic fitting. Aliases: `-fitt`, `--fit-target`.

Headroom the automatic fitter tries to leave on each device, in MiB.

- **Change:** Larger margins leave more VRAM unused by fitted model settings and may reduce offload/context; smaller margins fit more but leave less room for other work.
- **How much:** 1024 → 2048 asks for roughly one extra GiB of free margin per affected device. This is a fitting target, not an allocation or a hard reservation.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Minimum fitted context — `--fit-ctx`

Group: Attention & automatic fitting. Aliases: `-fitc`, `--fit-ctx`.

Smallest context the automatic fitter may choose.

- **Change:** Increase to protect a useful minimum context but make fitting harder; decrease to allow a smaller cache when VRAM is tight.
- **How much:** 4096 → 8192 doubles the context floor. For a dense cache the minimum KV payload can roughly double; allocation/backend overhead remains separate.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/src/llama-kv-cache.cpp), [official 3](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/docs/development/token_generation_performance_tips.md).

### Offload host tensor operations — `--op-offload`

Group: Backend buffers & operations. Aliases: `--op-offload`, `--no-op-offload`.

Allow eligible CPU-side tensor operations to run on a device.

- **Change:** On can accelerate supported operations but add transfers/device buffers; off keeps those operations on the host.
- **How much:** Benefit depends on operation size and backend. This is distinct from weight-layer count and KV placement.
- **Speed:** Moving weight computation to a sufficiently fast GPU can help; transfers or overfilled VRAM can make it slower.
- **VRAM:** Weight placement can change by gigabytes. Fit depends on tensor sizes, not only the number of layers.
- **RAM:** CPU-resident weights can fall when offloaded, but mapped pages/OS file cache may remain.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Activity → Logs → Native logging options

### Internal performance timings — `--perf`

Group: Server diagnostics. Aliases: `--perf`, `--no-perf`.

Collect internal libllama timing diagnostics.

- **Change:** On adds timing instrumentation; off avoids that measurement work. This does not tune CPU/GPU arithmetic.
- **How much:** Useful for comparisons; instrumentation can itself add overhead. Timings and verbose text logs are different settings.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Disable server logging — `--log-disable`

Group: Server diagnostics. Aliases: `--log-disable`.

Suppress the server's own diagnostic output.

- **Change:** On reduces diagnostic visibility and log I/O; leaving logs enabled helps diagnose model/load errors.
- **How much:** This does not stop API requests or free model weights. Launcher rotation already caps its captured streams; separate server files have their own behavior.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Additional server log file — `--log-file`

Group: Server diagnostics. Aliases: `--log-file`.

Write native server logs to the specified file.

- **Change:** A separate path adds direct file I/O and disk growth. It can contain information not covered by the launcher's captured-stream scrubbing.
- **How much:** This file is outside the launcher's 60 MiB rotating log cap. Choose an appropriate location and manage its size/privacy separately.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Machine-readable JSON logs — `--log-jsonl`

Group: Server diagnostics. Aliases: `--log-jsonl`, `--no-log-jsonl`.

Write one JSON log record per line to standard output.

- **Change:** On helps log parsers and disables color escapes; off uses the ordinary format.
- **How much:** Formatting changes bytes written, not model quality. High event volume still costs disk/CPU regardless of JSON formatting.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Log color policy — `--log-colors`

Group: Server diagnostics. Aliases: `--log-colors`.

Choose ANSI color output on, off or automatic.

- **Change:** Off removes terminal escape codes; on adds colors; auto follows the output target.
- **How much:** Usually negligible model-speed/RAM effect. File/GUI log readers often benefit from plain text.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### All diagnostic messages — `--verbose`

Group: Server diagnostics. Aliases: `-v`, `--verbose`, `--log-verbose`.

Enable the most detailed native server logging.

- **Change:** On can produce much more output and overhead; off returns to the configured verbosity threshold.
- **How much:** Useful temporarily for diagnosis. It is not a performance enhancement and can expose prompt/configuration detail in native outputs.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Log detail threshold — `--verbosity`

Group: Server diagnostics. Aliases: `-lv`, `--verbosity`, `--log-verbosity`.

Highest severity/detail level of native messages to keep.

- **Change:** Higher levels retain more detail and potentially more I/O; lower levels retain fewer messages.
- **How much:** The reviewed levels run 0 generic through 5 debug, with 3 info. Going 3 → 5 can grow logs substantially; not by a predictable fixed ratio.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Log message prefixes — `--log-prefix`

Group: Server diagnostics. Aliases: `--log-prefix`, `--no-log-prefix`.

Include context prefixes in native log messages.

- **Change:** On adds labeling useful for tracing; off removes that prefix text.
- **How much:** Small per-line formatting/disk overhead. It does not affect model parameter allocation.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Log timestamps — `--log-timestamps`

Group: Server diagnostics. Aliases: `--log-timestamps`, `--no-log-timestamps`.

Include time information in native log records.

- **Change:** On helps correlate events; off makes lines shorter.
- **How much:** Adds a timestamp per message, not a fixed inference delay. Overall overhead follows the number of emitted messages.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Prompt debugging directory — `--log-prompts-dir`

Group: Server diagnostics. Aliases: `--log-prompts-dir`.

Write prompt-debug information to a separate directory.

- **Change:** Enabling it adds disk writes and retains prompt content; leaving it unset avoids this debug output.
- **How much:** These files are outside launcher stream rotation/redaction and can contain private conversations. Use only for deliberate debugging and manage their disk/privacy separately.
- **Speed:** Detailed output can add formatting and disk I/O overhead; disabling it is not a guaranteed inference speedup.
- **VRAM:** Normally no direct weight/KV allocation change.
- **RAM:** Usually modest buffering; files consume disk. Separate server log files are outside launcher rotation/redaction.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Models → Model sources, adapters & media options

### Validate tensor values on load — `--check-tensors`

Group: Model sources. Aliases: `--check-tensors`.

Check model tensors for invalid values during loading.

- **Change:** On adds startup validation work; off skips that explicit scan. It is mainly a diagnostic rather than a speed setting.
- **How much:** Large files can take longer to check. It does not repair corrupt weights or prove that outputs are factually correct.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### GGUF metadata overrides — `--override-kv`

Group: Model sources. Aliases: `--override-kv`.

Override named model metadata values, not attention KV-cache entries.

- **Change:** Changes can alter tokenization or architecture behavior; wrong values can prevent load or degrade answers.
- **How much:** This is not a cache-size control despite its name. Use only exact metadata overrides documented for the model.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### LoRA adapter files — `--lora`

Group: Adapters & steering. Aliases: `--lora`.

Load small learned changes on top of a compatible base model.

- **Change:** More adapters add adapter tensors and potentially inference work; removing them returns toward base-model behavior.
- **How much:** Adapters must match the base model. Several 100 MiB adapters add their own data, not full copies of the base model.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### LoRA adapter strength — `--lora-scaled`

Group: Adapters & steering. Aliases: `--lora-scaled`.

Load adapters with explicit influence strengths.

- **Change:** Higher absolute strength applies a stronger learned change; 0 neutralizes its influence but may leave its tensors loaded. Smaller strength moves toward base behavior.
- **How much:** 0.5 versus 1.0 is half the adapter update coefficient, not half its RAM or a guaranteed halfway output.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Control vector files — `--control-vector`

Group: Adapters & steering. Aliases: `--control-vector`.

Load vectors that steer hidden model activations in a chosen direction.

- **Change:** More vectors add steering data and change output behavior; they do not retrain or replace the base weights.
- **How much:** Compatibility and vector training matter. This is a behavioral steering feature, not a general quality or speed upgrade.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Control vector strength — `--control-vector-scaled`

Group: Adapters & steering. Aliases: `--control-vector-scaled`.

Apply a steering vector with a specified coefficient.

- **Change:** Larger absolute values steer more strongly; smaller values steer less. Negative values can reverse the learned direction.
- **How much:** A coefficient of 0 may stop its influence while loaded data remains. Doubling strength does not double memory or guarantee a linear response.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Steered layer range — `--control-vector-layer-range`

Group: Adapters & steering. Aliases: `--control-vector-layer-range`.

Choose which inclusive model-layer range receives the steering vectors.

- **Change:** A wider range affects more layers; a narrower range localizes the steering. Quality effects are model/vector specific.
- **How much:** START=10 END=19 targets ten numbered layers. This is unrelated to how many layers are offloaded to GPU.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Main GGUF model — `--model`

Group: Model sources. Aliases: `-m`, `--model`.

Select the base model weights used for ordinary inference.

- **Change:** Larger or higher-precision models often need more memory and compute; smaller/lower-precision ones often fit and run more easily, with quality tradeoffs.
- **How much:** Use the Models picker for local files. Parameter count alone does not give file size: 8B at nominal 4 bits is roughly 4 GB of raw parameter bits before scales/other tensors, not an exact GGUF/VRAM estimate.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Direct model download URL — `--model-url`

Group: Model sources. Aliases: `-mu`, `--model-url`.

Tell llama.cpp to obtain its model from a URL rather than only a local file.

- **Change:** A different URL may choose different weights/quantization. Downloading adds network/disk time; the URL itself has no inference speed effect.
- **How much:** An 8 GiB file takes at least about 80 s at an actual 100 MiB/s transfer rate, before verification/load overhead. A remote source can take precedence over the local picker.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Docker model repository — `--docker-repo`

Group: Model sources. Aliases: `-dr`, `--docker-repo`.

Choose a model/quantization from a supported Docker Hub model repository.

- **Change:** Repository/tag selects the actual model; it is not the container runtime for MCP tools.
- **How much:** A tag can refer to a very different model size. Inspect the resolved model and quantization; this can trigger network downloads at launch.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Hugging Face model repository — `--hf-repo`

Group: Model sources. Aliases: `-hf`, `-hfr`, `--hf-repo`.

Choose a repository and optional quantization for llama.cpp's own model download.

- **Change:** Changing quantization/model changes weight size and possible quality/speed; a repository name alone is not a memory setting.
- **How much:** Specify user/repo and the supported quant suffix. This starts a server-managed source; the launcher's Models download manager is a separate workflow.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Exact Hugging Face GGUF file — `--hf-file`

Group: Model sources. Aliases: `-hff`, `--hf-file`.

Choose a specific repository file rather than relying on automatic quantization selection.

- **Change:** Selecting a smaller quantized file usually lowers weight memory; larger precision files usually require more memory. Quality/speed depend on model/backend.
- **How much:** This overrides automatic file selection for hf-repo. Ensure split shards/companions are available for the selected model.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Server-side Hugging Face token — `--hf-token`

Group: Model sources. Aliases: `-hft`, `--hf-token`.

Credential llama.cpp uses for its own restricted repository downloads.

- **Change:** A valid token can grant access; changing its text does not accelerate model inference or change quantization.
- **How much:** The launcher account token is separate from this server flag. Keep it private; credentials are omitted from public JSON and scrubbed from known display/log surfaces.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Use cached models only — `--offline`

Group: Model sources. Aliases: `--offline`.

Prevent llama.cpp's model retrieval from using network access.

- **Change:** On requires necessary files already cached/local; off permits supported downloads. This does not sandbox unrelated external tool processes.
- **How much:** Offline avoids download waits only when files are already present; it does not make a missing model available or reduce its memory use.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Multimodal projector file — `--mmproj`

Group: Images & video. Aliases: `-mm`, `--mmproj`.

Load the companion encoder/projector required for a supported vision/audio model.

- **Change:** Selecting a companion enables that model's media path; it adds separate weights/work. Omitting an incompatible projector does not turn arbitrary text weights into a vision model.
- **How much:** Use the companion intended for the base model. Its file/tensors add to the base model's memory, with placement controlled separately.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Projector download URL — `--mmproj-url`

Group: Images & video. Aliases: `-mmu`, `--mmproj-url`.

Download a compatible multimodal companion instead of providing its local path.

- **Change:** Changes download/setup, while the selected projector determines memory and encoder behavior.
- **How much:** Projector download size is separate from GGUF text weights. URLs may need network access at launch.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Automatically select a projector — `--mmproj-auto`

Group: Images & video. Aliases: `--mmproj-auto`, `--no-mmproj`, `--no-mmproj-auto`.

Allow automatic companion selection when the source/runtime supports it.

- **Change:** On can fetch/load an available companion; off avoids that automatic projector selection and can disable required media capability.
- **How much:** This can add a separate model download/allocation. Check which companion is resolved rather than assuming it is free.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Run the media projector on GPU — `--mmproj-offload`

Group: Images & video. Aliases: `--mmproj-offload`, `--no-mmproj-offload`.

Control GPU offload of the companion encoder/projector.

- **Change:** On can improve image/media encoding and consumes GPU memory; off puts eligible work in CPU/RAM and may be slower.
- **How much:** This does not change the text model's GPU layer count. The saving is the eligible projector/buffer allocation, not a fixed fraction of text weights.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Projector device — `--mmproj-device`

Group: Images & video. Aliases: `-mmdev`, `--mmproj-device`.

Choose where the companion encoder/projector is placed.

- **Change:** Selecting another GPU can rebalance VRAM but add transfers; none requests CPU placement where supported.
- **How much:** Use an actual device name reported by the executable. Main model and projector need not occupy the same GPU.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Minimum image token budget — `--image-min-tokens`

Group: Images & video. Aliases: `--image-min-tokens`.

Minimum visual token count requested for dynamic-resolution vision models.

- **Change:** Increase to retain more image detail and increase encoding/prompt work; decrease to permit smaller representations.
- **How much:** 256 → 512 doubles the minimum token budget, not necessarily resolution or accuracy. Only dynamic-resolution models using this option are affected.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Maximum image token budget — `--image-max-tokens`

Group: Images & video. Aliases: `--image-max-tokens`.

Upper limit on visual tokens for supported dynamic-resolution vision models.

- **Change:** Increase to allow more detail and more work/cache use; decrease to cap media cost but possibly miss small details.
- **How much:** 512 → 1024 doubles the allowed image-token ceiling. It does not double model weights or guarantee double image accuracy.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Image encoding microbatch — `--mtmd-batch-max-tokens`

Group: Images & video. Aliases: `--mtmd-batch-max-tokens`.

Maximum image tokens encoded in one media-processing batch.

- **Change:** Increase for more parallel encoder work and potentially larger buffers; decrease to lower temporary memory and use more passes.
- **How much:** 1024 → 2048 doubles the batch-token ceiling, not the image's final token count. Actual buffer growth depends on the encoder/backend.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Frames analyzed per second — `--video-fps`

Group: Images & video. Aliases: `--video-fps`.

Target sampling rate for video input.

- **Change:** Higher FPS analyzes more frames and costs more decoding/encoding/prompt work; lower FPS is cheaper but can miss short events.
- **How much:** For 10 seconds of video, 2 FPS selects about 20 frames and 4 FPS about 40 before decoder rounding/limits. Frame count can approximately double; speed/accuracy do not necessarily do so.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Timestamp spacing — `--video-timestamp-interval`

Group: Images & video. Aliases: `--video-timestamp-interval`.

Milliseconds between inserted text timestamps for video.

- **Change:** Smaller intervals add more timestamp text; larger intervals use fewer markers and coarser time cues.
- **How much:** 1000 ms inserts markers about every second; 5000 about every five seconds. This does not set video FPS or alter the model's weights.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Video decoder directory — `--video-ffmpeg-dir`

Group: Images & video. Aliases: `--video-ffmpeg-dir`.

Where llama.cpp finds ffmpeg and ffprobe for supported video decoding.

- **Change:** Changing the directory chooses installed tools, not a quality or speed multiplier.
- **How much:** Use a compatible local installation. The actual codec/resolution, decoder and sampled frame count determine CPU/RAM cost.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### API model aliases — `--alias`

Group: Router models. Aliases: `-a`, `--alias`.

Names clients can use to identify this model through the API.

- **Change:** Adding aliases offers alternate names for the same configured model; it does not create more model instances.
- **How much:** Three aliases do not allocate three copies of weights. In router presets use unique names to avoid ambiguous selection.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Informational model tags — `--tags`

Group: Router models. Aliases: `--tags`.

Attach descriptive labels to the model listing.

- **Change:** More tags improve organization but do not control routing or inference behavior.
- **How much:** Tags are metadata. They do not alter memory, compute or automatically select models for requests.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allowed local media directory — `--media-path`

Group: Images & video. Aliases: `--media-path`.

Base directory for local file media references used by server requests.

- **Change:** Enabling a directory makes those supported relative media files available; a broader directory does not make encoding faster.
- **How much:** Choose only intended media files. Input sizes/frame counts determine memory; this option controls file access rather than resolution.
- **Speed:** More image tokens or video frames add encoder and prompt-processing work, increasing time to first token.
- **VRAM:** Projector weights and encoder/attention buffers can add VRAM; changing resolution does not resize base text weights.
- **RAM:** Decoded images, frames/audio and input buffers add RAM; long media can be substantial.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/mtmd/README.md).

### Router models directory — `--models-dir`

Group: Router models. Aliases: `--models-dir`.

Directory advertised by the on-demand model router.

- **Change:** More available files add choices; they do not all load immediately. Use the Models folder control for this launcher's directory.
- **How much:** One directory containing ten models is not ten loaded models. Simultaneous loaded memory is controlled by router policy/workload and models-max.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Router preset file — `--models-preset`

Group: Router models. Aliases: `--models-preset`.

INI configuration of per-model router settings.

- **Change:** Allows each routed model to use its own model/runtime choices; changing presets can substantially change per-model allocations.
- **How much:** A small INI file can describe many gigabytes of models. Confirm paths and per-model context/offload values in the preset.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Maximum loaded router models — `--models-max`

Group: Router models. Aliases: `--models-max`.

How many model workers the router may keep loaded simultaneously.

- **Change:** Increase to reduce model switching/unloading but permit much higher total RAM/VRAM; decrease to constrain concurrent loaded models. 0 means unlimited in the reviewed build.
- **How much:** If two loaded workers each need 6 GiB, together they need roughly 12 GiB plus router/overheads. This counts models, not request slots within one model.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Load routed models on demand — `--models-autoload`

Group: Router models. Aliases: `--models-autoload`, `--no-models-autoload`.

Let an incoming request trigger loading an advertised model.

- **Change:** On is convenient but the first request can wait for loading; off requires explicit model loading before use.
- **How much:** A cold model can take seconds or longer depending on file/device speed. This is different from automatically loading every listed model at startup.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Load adapters inactive — `--lora-init-without-apply`

Group: Adapters & steering. Aliases: `--lora-init-without-apply`.

Load adapter data but wait for a later API action before applying it.

- **Change:** On retains adapter memory without immediately changing inference; off applies loaded adapters normally.
- **How much:** Inactive adapters can still consume memory. Use when deliberately switching adapter influence through the server API.
- **Speed:** Adapters add loading work and may add inference operations; effect depends on rank/count/backend.
- **VRAM:** Adapter tensors add memory if placed on GPU; strength alone normally does not change their stored size.
- **RAM:** Adapter/vector files and tensors add host memory; strength does not duplicate the base model.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Unload after idle time — `--sleep-idle-seconds`

Group: Idle model unloading. Aliases: `--sleep-idle-seconds`.

How long a model may sit idle before entering its supported sleep/unload state.

- **Change:** A shorter timeout can release idle resources sooner but trigger more reload delays; a longer one keeps warm memory longer. -1 disables sleeping in the reviewed build.
- **How much:** 60 → 300 waits five times longer before sleep, not five times the inference speed. Freed GPU/RAM and wake time depend on backend, mapping and model size.
- **Speed:** Primarily changes startup, disk access and paging; it is not a fixed tokens/second multiplier.
- **VRAM:** For unchanged offload/model settings, weight VRAM usually stays similar.
- **RAM:** Mapped address space is not resident RAM. Locked pages cannot be paged out; lazy reads trade RAM for disk access.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### EmbeddingGemma preset — `--embd-gemma-default`

Group: Model download presets. Aliases: `--embd-gemma-default`.

Request the upstream default EmbeddingGemma setup.

- **Change:** Selects a particular embedding-model preset and may download weights. It does not turn the current chat model into an embedding model.
- **How much:** Check the resolved files/quantization and embedding configuration; the preset name is not a memory-fit guarantee.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen Coder 1.5B preset — `--fim-qwen-1.5b-default`

Group: Model download presets. Aliases: `--fim-qwen-1.5b-default`.

Request the upstream small Qwen 2.5 Coder infill preset.

- **Change:** Smaller weight models often fit more easily and evaluate faster than larger members, with capability tradeoffs.
- **How much:** 1.5B is nominal parameter count, not 1.5 GiB memory. Quantization, cache, buffers and actual files determine allocation.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen Coder 3B preset — `--fim-qwen-3b-default`

Group: Model download presets. Aliases: `--fim-qwen-3b-default`.

Request the upstream Qwen 2.5 Coder 3B infill model preset.

- **Change:** Changes model choice and may download files; compare its coding quality and measured speed with smaller/larger candidates.
- **How much:** 3B is about twice 1.5B parameters, not a guarantee of twice the total RAM or half the speed across different configurations.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen Coder 7B preset — `--fim-qwen-7b-default`

Group: Model download presets. Aliases: `--fim-qwen-7b-default`.

Request the upstream Qwen 2.5 Coder 7B infill model preset.

- **Change:** A larger model generally needs more weight storage/work, with possible capability gains; it can exceed a smaller GPU's budget.
- **How much:** A nominal 7B 4-bit parameter payload is about 3.5 GB before quantization scales/other tensors and runtime memory. Inspect actual files, not just this arithmetic.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen 7B speculative infill preset — `--fim-qwen-7b-spec`

Group: Model download presets. Aliases: `--fim-qwen-7b-spec`.

Request the 7B target plus the upstream smaller draft setup.

- **Change:** Can improve coding throughput when proposals match, but adds draft weights/cache/compute and can be slower with poor acceptance.
- **How much:** Budget for both models plus context/buffers. This is not the same memory footprint as a standalone 7B model.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen 14B speculative infill preset — `--fim-qwen-14b-spec`

Group: Model download presets. Aliases: `--fim-qwen-14b-spec`.

Request the upstream 14B target and draft infill setup.

- **Change:** Larger target weights increase memory/work; drafting adds its own costs and only helps with suitable acceptance.
- **How much:** The preset's model count/quantization matters. It is not guaranteed to fit or run faster simply because speculation is enabled.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Qwen Coder 30B A3B preset — `--fim-qwen-30b-default`

Group: Model download presets. Aliases: `--fim-qwen-30b-default`.

Request the upstream Qwen3 Coder 30B A3B preset.

- **Change:** MoE activates only part of the parameters for each token, but all required expert weights still need storage/placement.
- **How much:** 30B total and roughly 3B active does not mean only a dense 3B model's weight memory. Check the MoE GGUF and expert placement.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### gpt-oss 20B preset — `--gpt-oss-20b-default`

Group: Model download presets. Aliases: `--gpt-oss-20b-default`.

Request the upstream gpt-oss 20B model preset.

- **Change:** Selects different architecture/weights and may download them. Quality, active compute and cache behavior are model-specific.
- **How much:** The name is total nominal parameter scale, not a VRAM figure. Consult the resolved file size and model resource estimate.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### gpt-oss 120B preset — `--gpt-oss-120b-default`

Group: Model download presets. Aliases: `--gpt-oss-120b-default`.

Request the upstream larger gpt-oss model preset.

- **Change:** A much larger total weight set can need substantial RAM/VRAM even when only some experts are active.
- **How much:** 120B is not 120 GiB by definition, and active parameters do not eliminate storage of other required experts. Inspect quantization and placement.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Gemma 3 4B vision preset — `--vision-gemma-4b-default`

Group: Model download presets. Aliases: `--vision-gemma-4b-default`.

Request the upstream smaller Gemma 3 vision model setup.

- **Change:** Adds the supported vision path/projector as well as text weights; media encoding adds work beyond text-only generation.
- **How much:** Do not budget from text GGUF alone. Projector, image tokens, context and backend buffers need additional memory.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Gemma 3 12B vision preset — `--vision-gemma-12b-default`

Group: Model download presets. Aliases: `--vision-gemma-12b-default`.

Request the upstream larger Gemma 3 vision model setup.

- **Change:** Usually requires more weight memory/work than the smaller preset, plus projector/media processing; quality/speed are workload dependent.
- **How much:** 12B versus 4B is about 3× nominal parameters, not a universal 3× RAM or 1/3 speed prediction for the complete application.
- **Speed:** Architecture, size, quantization and storage determine load time and inference speed; the path/name itself has no speed multiplier.
- **VRAM:** A different model can change weight/KV VRAM by gigabytes. Check the selected GGUF and offload settings.
- **RAM:** CPU weights/mapped files depend on the chosen model. Downloaded files also consume disk space.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Settings → Sampling

### Sampler pipeline order — `--samplers`

Group: Randomness & candidate selection. Aliases: `--samplers`.

Which token-selection filters run, and in what sequence.

- **Change:** Adding, removing or reordering filters changes the final distribution; there is no universal better order.
- **How much:** top-k then top-p need not behave like top-p then top-k. Use semicolon-separated names supported by the selected executable.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Sampling random seed — `--seed`

Group: Randomness & candidate selection. Aliases: `-s`, `--seed`.

Choose a repeatable pseudo-random sampling sequence for comparison.

- **Change:** Changing the integer chooses another random sequence, not more or less randomness. The special random-seed value requests a fresh sequence.
- **How much:** Use one fixed seed when comparing settings. Identical seeds do not guarantee bit-identical output across backends, parallel workloads or versions.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Compact sampler sequence — `--sampler-seq`

Group: Randomness & candidate selection. Aliases: `--sampler-seq`, `--sampling-seq`.

Short-letter notation for the sampling pipeline.

- **Change:** Changing letters/order changes the same token-filtering process; it is not a CPU scheduling sequence.
- **How much:** Use this or the named sampler pipeline deliberately. Letter meanings and defaults are version-specific; check the executable help.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Temperature — `--temp`

Group: Randomness & candidate selection. Aliases: `--temp`, `--temperature`.

How strongly sampling favors high-probability tokens over alternatives.

- **Change:** Higher positive values flatten the distribution and encourage variety; lower values favor likely tokens. Very low/zero settings can select greedily.
- **How much:** 0.4 → 0.8 is more randomness, not twice the creativity or half the quality. Effects depend on model and the other active samplers.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Top-K candidates — `--top-k`

Group: Randomness & candidate selection. Aliases: `--top-k`.

Keep at most K of the highest-ranked token candidates.

- **Change:** Higher K permits more alternatives; lower K narrows choice. 0 disables this filter in the reviewed runtime.
- **How much:** K=1 leaves one candidate and is effectively greedy at this stage; K=40 permits up to forty. This does not change model size.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Top-P probability mass — `--top-p`

Group: Randomness & candidate selection. Aliases: `--top-p`.

Keep the smallest high-ranked candidate set reaching the requested probability mass.

- **Change:** Higher values retain more probability mass and usually more alternatives; lower values make selection narrower. 1 disables this filter.
- **How much:** 0.90 retains about 90% cumulative probability before later selection, not 90% of all vocabulary tokens.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Relative minimum probability — `--min-p`

Group: Randomness & candidate selection. Aliases: `--min-p`.

Discard candidates too unlikely relative to the most probable token.

- **Change:** Higher values filter more aggressively; lower values keep more alternatives. 0 disables it.
- **How much:** At min-p 0.05 and a top token probability of 0.40, the cutoff is about 0.02 before renormalization. It is relative, not an absolute 5% threshold.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Logit standard-deviation filter — `--top-nsigma`

Group: Randomness & candidate selection. Aliases: `--top-nsigma`, `--top-n-sigma`.

Limit candidates using distance from the top logit measured in standard deviations.

- **Change:** A larger allowed distance generally retains more candidates; a smaller one narrows the set. Negative disables it in the reviewed build.
- **How much:** This is a distribution-dependent cutoff, not a token count or a memory size. Test output quality with your other filters.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### XTC activation probability — `--xtc-probability`

Group: Randomness & candidate selection. Aliases: `--xtc-probability`.

How often the exclude-top-choices sampler is applied.

- **Change:** Increase to apply its diversity-inducing exclusion more often; decrease toward 0 to disable it.
- **How much:** 0.1 → 0.5 changes activation from roughly 10% to 50% of sampling decisions, not a speed multiplier; output can become less predictable.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### XTC exclusion threshold — `--xtc-threshold`

Group: Randomness & candidate selection. Aliases: `--xtc-threshold`.

Probability threshold used to decide which very likely choices XTC can exclude.

- **Change:** Lower thresholds can make more top candidates eligible for exclusion; higher thresholds make exclusion harder. 1 disables it.
- **How much:** Interactively tune with XTC probability. Aggressive exclusion can hurt coherence; memory remains dominated by weights/KV.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Locally typical sampling — `--typical`

Group: Randomness & candidate selection. Aliases: `--typical`, `--typical-p`.

Keep candidates whose information content is typical for the current distribution.

- **Change:** Lower p restricts retained probability mass more; p near 1 leaves more alternatives. 1 disables it.
- **How much:** Typicality differs from simply choosing the most probable tokens. A value of 0.9 is not a 10% quality/performance change.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Repetition history window — `--repeat-last-n`

Group: Repetition control. Aliases: `--repeat-last-n`.

Number of recent tokens considered by repetition penalties.

- **Change:** More history catches repeats farther back and adds history checking; less focuses on recent text. 0 disables this history-based penalty window.
- **How much:** 64 → 128 doubles inspected token positions, not total model memory. Excessive penalties can prevent legitimate repeated names/code.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Repeated-token penalty — `--repeat-penalty`

Group: Repetition control. Aliases: `--repeat-penalty`.

Multiplicative adjustment discouraging recently used tokens.

- **Change:** Above 1 penalizes repeats more; 1 is neutral; values below 1 can encourage repetition.
- **How much:** 1.1 → 1.2 is a change in the penalty formula, not a guaranteed 10% repetition reduction. Test prose and code separately.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Penalty for appearing at all — `--presence-penalty`

Group: Repetition control. Aliases: `--presence-penalty`.

Add a penalty to a token if it has appeared in the considered history.

- **Change:** More positive values discourage any reuse; zero is neutral; negative values can encourage reuse.
- **How much:** Each present token gets the configured adjustment independent of its count. The impact depends on model logits, not a fixed percentage.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Penalty per occurrence — `--frequency-penalty`

Group: Repetition control. Aliases: `--frequency-penalty`.

Penalize tokens in proportion to how often they occur in the considered history.

- **Change:** Higher positive values increasingly discourage frequent tokens; zero is neutral; negative values encourage them.
- **How much:** At strength 0.2, five counted occurrences contribute about a 1.0 logit adjustment before other sampler operations. This is not a quality guarantee.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### DRY repetition strength — `--dry-multiplier`

Group: Repetition control. Aliases: `--dry-multiplier`.

Overall strength of the penalty against repeated text sequences.

- **Change:** Increase to suppress long repeated passages more strongly; decrease toward 0 to disable DRY.
- **How much:** A strength of 2 applies twice the configured penalty magnitude of 1 for otherwise identical inputs, not twice the text quality.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### DRY penalty growth — `--dry-base`

Group: Repetition control. Aliases: `--dry-base`.

Base controlling how quickly repeated-sequence penalties grow beyond the allowed length.

- **Change:** Higher bases make long repetitions increasingly expensive; lower bases make growth gentler.
- **How much:** For the exponential component, base 2 grows by 2× per additional penalized token. Other DRY settings and detected matches still matter.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Unpenalized repeated length — `--dry-allowed-length`

Group: Repetition control. Aliases: `--dry-allowed-length`.

How long a matching sequence may repeat before DRY penalizes it.

- **Change:** Increase to tolerate longer repeated phrases; decrease to start penalizing sooner.
- **How much:** 2 → 4 allows two additional repeated tokens before this sequence penalty begins. Code/structured output may require legitimate repetition.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### DRY history window — `--dry-penalty-last-n`

Group: Repetition control. Aliases: `--dry-penalty-last-n`.

How much preceding text DRY searches for repeated sequences.

- **Change:** Increase to catch farther-back repetitions with more history work; decrease to limit its search. 0 disables the configured DRY penalty window.
- **How much:** 64 → 256 expands the token-history ceiling fourfold, not model memory fourfold. Special negative values follow the installed version's help.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### DRY sequence boundaries — `--dry-sequence-breaker`

Group: Repetition control. Aliases: `--dry-sequence-breaker`.

Strings that split repetition matching into separate sequences.

- **Change:** Adding breakers stops DRY matching across those boundaries; removing them permits longer cross-boundary matches.
- **How much:** A newline breaker can separate paragraphs. Setting custom breakers replaces the defaults; none disables the breaker list rather than DRY itself.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Adaptive-P target probability — `--adaptive-target`

Group: Adaptive sampling. Aliases: `--adaptive-target`.

Guide adaptive sampling toward tokens near a chosen probability.

- **Change:** A higher/lower target changes which part of the candidate distribution is favored; negative disables the method.
- **How much:** Valid targets are 0–1 in the reviewed version. They are probability targets, not temperature, a quality score or a memory fraction.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Adaptive-P smoothing — `--adaptive-decay`

Group: Adaptive sampling. Aliases: `--adaptive-decay`.

How much the adaptive target retains its previous state.

- **Change:** Higher values are steadier/slower to react; lower values respond more quickly to recent changes.
- **How much:** 0.9 retains much more previous state than 0.1 at each update. This does not mean 90% of model memory is cached.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Dynamic temperature range — `--dynatemp-range`

Group: Adaptive sampling. Aliases: `--dynatemp-range`.

Allow sampling temperature to vary around its base value.

- **Change:** A larger range permits larger changes between distributions; 0 disables dynamic temperature.
- **How much:** With base 0.8 and range 0.2, the intended range is around 0.6–1.0 before implementation limits; this is randomness variation, not an inference speed range.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Dynamic temperature response curve — `--dynatemp-exp`

Group: Adaptive sampling. Aliases: `--dynatemp-exp`.

Shape how distribution uncertainty maps to the dynamic temperature range.

- **Change:** Changing the exponent changes the response nonlinearly; it is not another temperature limit.
- **How much:** At zero dynamic range it has no useful effect. Compare generated text with fixed seeds rather than reading it as a speed/power exponent.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Mirostat sampling mode — `--mirostat`

Group: Adaptive sampling. Aliases: `--mirostat`.

Choose an adaptive entropy controller instead of the usual candidate filters.

- **Change:** 0 disables it; 1 and 2 choose different algorithms, not progressively higher strength. Some normal filters are ignored while it is active.
- **How much:** Do not compare mode 2 as twice mode 1. Tune target entropy and learning rate, and check the sampler interactions in executable help.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Mirostat learning rate — `--mirostat-lr`

Group: Adaptive sampling. Aliases: `--mirostat-lr`.

How quickly Mirostat changes its controller after a token.

- **Change:** Higher rates react faster but can oscillate; lower rates adapt more slowly and smoothly.
- **How much:** 0.1 → 0.2 doubles the update coefficient, not generation speed or target entropy.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Mirostat target entropy — `--mirostat-ent`

Group: Adaptive sampling. Aliases: `--mirostat-ent`.

Target surprise/uncertainty level of generated tokens.

- **Change:** Higher targets allow less predictable choices; lower targets favor more predictable output.
- **How much:** The parameter is an entropy target, not token count. It does not change the model's context capacity or parameter memory.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Token probability bias — `--logit-bias`

Group: Explicit token preferences. Aliases: `-l`, `--logit-bias`.

Add a positive/negative adjustment to specific token IDs before sampling.

- **Change:** More positive favors the token; more negative suppresses it. This targets a tokenizer ID, not arbitrary text or a percentage.
- **How much:** Before other filters, a +1 logit shift multiplies that token's relative odds by about e (2.72); renormalization and sampling still matter. Strong bans can break desired output.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Sample on the compute backend — `--backend-sampling`

Group: Randomness & candidate selection. Aliases: `-bs`, `--backend-sampling`.

Let the backend perform supported token sampling operations.

- **Change:** On can avoid some device-to-host sampling work; off follows the normal host path. Support and benefit vary.
- **How much:** Experimental in the reviewed build. It does not automatically accelerate model evaluation or reduce model-weight memory.
- **Speed:** Primarily changes which token is chosen, not model evaluation speed. Longer answers still take longer overall.
- **VRAM:** No direct change to model weights or reserved KV capacity.
- **RAM:** Sampler/history state is usually small compared with model memory; some history windows add CPU work.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Settings → API

### Embedding pooling method — `--pooling`

Group: Embedding & ranking tasks. Aliases: `--pooling`.

How token-level features are combined into a sequence embedding or ranking output.

- **Change:** none, mean, cls, last and rank choose different output semantics, not increasing quality levels.
- **How much:** Use the method expected by the model and retrieval pipeline. A mismatched pooling method can produce poor retrieval despite valid-looking vectors.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Embedding normalization — `--embd-normalize`

Group: Embedding & ranking tasks. Aliases: `--embd-normalize`.

How the output vector is scaled before being returned.

- **Change:** -1 disables normalization; 0/1/2 and larger values choose different normalization rules. This is not the vector dimension.
- **How much:** 2 uses Euclidean/L2 normalization; 1 uses L1. Match the similarity metric expected by your search index rather than assuming higher is better.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Share listener port — `--reuse-port`

Group: HTTP & browser access. Aliases: `--reuse-port`.

Allow supported socket arrangements to bind the same port.

- **Change:** On permits the runtime's reuse-port behavior; off preserves ordinary exclusive binding.
- **How much:** This does not share model weights or bypass the launcher's port-conflict preflight. Leave unset for a single launcher-owned server.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Static website directory — `--path`

Group: HTTP & browser access. Aliases: `--path`.

Serve static web files from a directory.

- **Change:** Changing the path changes served files, not the text model or GPU settings.
- **How much:** Keep secrets outside the served directory. Static file size affects disk/network work; this is not the models directory.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allowed browser origins — `--cors-origins`

Group: HTTP & browser access. Aliases: `--cors-origins`.

Which websites browsers may let call this server.

- **Change:** A narrower origin list restricts browser clients; wildcard is broader. It does not bind/unbind network interfaces.
- **How much:** Use exact browser origins including scheme/port as appropriate. CORS is a browser access policy, not API authentication or a firewall.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allowed browser HTTP methods — `--cors-methods`

Group: HTTP & browser access. Aliases: `--cors-methods`.

HTTP methods permitted by browser CORS checks.

- **Change:** More methods permit more kinds of cross-origin browser request; fewer may block legitimate clients.
- **How much:** Methods such as GET and POST are request types. This changes compatibility/access policy, not tokens/s.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allowed browser request headers — `--cors-headers`

Group: HTTP & browser access. Aliases: `--cors-headers`.

Headers a browser may send cross-origin.

- **Change:** A broader list supports more client headers; a narrower one can block Authorization/content-type use if omitted.
- **How much:** Ensure required API headers are allowed. It does not create an API key or increase connection capacity.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allow credentialed browser requests — `--cors-credentials`

Group: HTTP & browser access. Aliases: `--cors-credentials`, `--no-cors-credentials`.

Permit browser cross-origin credentials according to the runtime's CORS implementation.

- **Change:** On permits supported credentialed requests; off disallows that CORS behavior. Wildcard/origin reflection has version-specific behavior.
- **How much:** This is not the API-key value and does not enable TLS. Choose it to match the intended browser clients/origin policy.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### API path prefix — `--api-prefix`

Group: HTTP & browser access. Aliases: `--api-prefix`.

Serve endpoints under an extra URL path.

- **Change:** Changing it moves the API path and requires clients to use the same prefix; it does not change model computation.
- **How much:** With /llama and port 8080, clients use http://LAN-IP:8080/llama/... rather than the root path. Avoid a trailing slash as instructed by the executable.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Inline browser UI defaults — `--ui-config`

Group: Web UI configuration. Aliases: `--ui-config`, `--webui-config`.

JSON defaults for llama.cpp's browser chat interface.

- **Change:** Changes UI behavior/defaults recognized by that version; more JSON keys do not inherently use more model memory.
- **How much:** This configures the server's web page, not launcher's theme/profiles. Per-chat options may still change actual request behavior.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Browser UI defaults file — `--ui-config-file`

Group: Web UI configuration. Aliases: `--ui-config-file`, `--webui-config-file`.

Read browser UI defaults from a JSON file.

- **Change:** Same UI effect as inline configuration; file location is an organizational choice.
- **How much:** A UI default is not necessarily a hard server-side limit. Confirm which settings the installed server recognizes.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Built-in browser chat — `--ui`

Group: Web UI configuration. Aliases: `--ui`, `--webui`, `--no-ui`, `--no-webui`.

Enable llama.cpp's own browser interface.

- **Change:** On serves the web UI; off can leave supported API endpoints available without that interface.
- **How much:** This is the server's browser page, not this desktop launcher's GUI. It does not load a second copy of model weights.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Embedding-only service — `--embedding`

Group: Embedding & ranking tasks. Aliases: `--embedding`, `--embeddings`.

Restrict service to embedding use with a dedicated embedding model.

- **Change:** On selects an embedding workload instead of ordinary chat generation; off leaves the normal supported service behavior.
- **How much:** Use an embedding-trained model. Embedding throughput/sequence length are different metrics from generated chat tokens/s.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Reranking endpoint — `--rerank`

Group: Embedding & ranking tasks. Aliases: `--rerank`, `--reranking`.

Enable ranking candidate documents with a compatible reranking model.

- **Change:** More/longer candidates mean more sequence evaluation and latency; simply enabling the endpoint does not make chat answers better.
- **How much:** Ranking 100 comparable documents involves more work than ranking 10. Use the correct model and measure documents/s or request latency.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### API authentication keys — `--api-key`

Group: Authentication & TLS. Aliases: `--api-key`.

Keys clients must supply to authenticate model API requests.

- **Change:** Adding keys permits more credentials; changing key length/text does not change inference quality or speed.
- **How much:** Use client Authorization bearer credentials. This is separate from Hugging Face and MCP tokens; keep keys in private profiles rather than public exports.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### API key file — `--api-key-file`

Group: Authentication & TLS. Aliases: `--api-key-file`.

Load authentication keys from a local file, one per line.

- **Change:** More non-comment keys permit more credentials; the file does not load extra model copies.
- **How much:** Protect the file itself. The launcher reads a supported key for its own health/test requests; the file is not embedded into a shareable export.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### HTTPS private key file — `--ssl-key-file`

Group: Authentication & TLS. Aliases: `--ssl-key-file`.

PEM private key used by server-side TLS.

- **Change:** Enabling a matching key/certificate changes connections from HTTP to HTTPS and adds TLS work, not model weight memory.
- **How much:** Use it with the matching certificate. Clients must trust that certificate; the launcher does not bypass certificate verification.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### HTTPS certificate file — `--ssl-cert-file`

Group: Authentication & TLS. Aliases: `--ssl-cert-file`.

PEM certificate paired with the TLS private key.

- **Change:** Changes encryption/identity setup; using a larger certificate chain is not an inference optimization.
- **How much:** A valid matching pair and client trust are required. Do not put the private key in a served static-files directory.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### HTTP read/write timeout — `--timeout`

Group: HTTP & browser access. Aliases: `-to`, `--timeout`.

Seconds allowed by the server's HTTP read/write timeout policy.

- **Change:** Increase to tolerate slow clients/longer waits; decrease to free stalled connections sooner but risk failed requests.
- **How much:** 60 → 120 doubles this timeout allowance, not response speed or the generation token budget. Streaming/inference timeout semantics vary by runtime.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Streaming heartbeat interval — `--sse-ping-interval`

Group: HTTP & browser access. Aliases: `--sse-ping-interval`.

Seconds between keepalive pings on supported SSE streams.

- **Change:** Smaller positive intervals send more keepalives; larger ones send less traffic but may exceed proxy idle limits. -1 disables them in the reviewed build.
- **How much:** 30 → 10 sends roughly three times as many heartbeat opportunities. Heartbeats are not generated text and do not increase model tokens/s.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### HTTP request workers — `--threads-http`

Group: HTTP & browser access. Aliases: `--threads-http`.

Workers handling HTTP requests rather than the model's token calculations.

- **Change:** Increase for more concurrent HTTP handling; decrease to save worker overhead. Too many can create contention.
- **How much:** 4 → 8 is eight requested HTTP workers, not eight model inference threads or eight loaded models. Consider parallel slots separately.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Prometheus metrics endpoint — `--metrics`

Group: Monitoring & mutable endpoints. Aliases: `--metrics`.

Expose server measurements such as request/token performance.

- **Change:** On enables monitoring data; off avoids that endpoint. Scraping often adds some request/formatting work.
- **How much:** Metrics measure the workload; they do not make it faster. The launcher can display available metrics when enabled.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Allow global property updates — `--props`

Group: Monitoring & mutable endpoints. Aliases: `--props`.

Permit supported global settings to be changed through POST /props.

- **Change:** On exposes mutable server properties; off keeps that mutation endpoint disabled.
- **How much:** Enabling it does not apply a speed preset. Use appropriate API access control because clients can change supported global behavior.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Request-slot monitoring — `--slots`

Group: Monitoring & mutable endpoints. Aliases: `--slots`, `--no-slots`.

Expose information about the server's request slots.

- **Change:** On allows compatible clients to inspect slot state; off removes that monitoring endpoint.
- **How much:** Four slots are not four loaded weight models. Slot visibility does not itself increase concurrency or context allocation.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Launch

### Server listen address — `--host`

Group: Listen address. Aliases: `--host`.

Which local IP interfaces accept server connections.

- **Change:** 127.0.0.1 limits access to this computer; 0.0.0.0 listens on all IPv4 interfaces; a specific local IP selects that interface.
- **How much:** Set this in Launch. Other devices connect to the computer's real LAN IP, not 0.0.0.0. This changes reachability, not model speed.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Server listen port — `--port`

Group: Listen address. Aliases: `--port`.

TCP port used by the server listener.

- **Change:** Changing the number selects another listener port; a higher number is not faster.
- **How much:** Set this in Launch. For port 8080 clients use http://LAN-IP:8080; choose an available port in the launcher's accepted range.
- **Speed:** Changes connectivity, API behavior or request handling rather than per-token model arithmetic.
- **VRAM:** No direct weight or KV-capacity change.
- **RAM:** Connections, TLS and request buffers add workload-dependent overhead.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

## Connections → Advanced native tools & MCP options

### Browser MCP CORS proxy — `--ui-mcp-proxy`

Group: Native tools & MCP overrides. Aliases: `--ui-mcp-proxy`, `--webui-mcp-proxy`, `--no-ui-mcp-proxy`, `--no-webui-mcp-proxy`.

Enable the server web UI's experimental proxy to MCP endpoints.

- **Change:** On allows the proxy behavior and its network work; off keeps it disabled. It does not set the launcher's MCP URL.
- **How much:** This adds a network access capability. The upstream documentation cautions against enabling it in untrusted environments.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Built-in server tools — `--tools`

Group: Native tools & MCP overrides. Aliases: `--tools`.

Enable selected built-in agent tools for model use.

- **Change:** More enabled tools expand available actions and prompt/tool metadata; they can trigger external work when used.
- **How much:** Enabling a shell/file tool grants capabilities, not free model intelligence. This is distinct from merely listing tools in the launcher's MCP panel.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Tool execution environment — `--tools-runtime`

Group: Native tools & MCP overrides. Aliases: `--tools-runtime`.

Choose whether built-in tools execute on the host or in a supported container/runtime.

- **Change:** Containers can add startup and process overhead; host execution avoids that layer but has different access boundaries.
- **How much:** docker:... or podman:... describes tool execution, not model weight installation. Runtime images/processes consume their own disk/RAM.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Native MCP configuration file — `--mcp-servers-config`

Group: Native tools & MCP overrides. Aliases: `--mcp-servers-config`.

Provide raw MCP server definitions to llama.cpp.

- **Change:** More configured servers can add tool metadata, connections and subprocesses; fewer reduce that external surface/work.
- **How much:** Use either the launcher's connection form or these raw server flags, not both for the same launch. Testing discovery does not execute tools.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Inline native MCP definitions — `--mcp-servers-json`

Group: Native tools & MCP overrides. Aliases: `--mcp-servers-json`.

Provide MCP server definitions as an inline JSON value.

- **Change:** Same integration effect as a config file; inline credentials are still secrets and must stay private.
- **How much:** Use the structured Connections form for the normal workflow. This raw override is omitted from public export and redacted in known previews/logs.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).

### Built-in agent capabilities preset — `--agent`

Group: Native tools & MCP overrides. Aliases: `-ag`, `--agent`, `-no-ag`, `--no-agent`.

Enable the upstream agent preset including tools/proxy capabilities.

- **Change:** On expands the actions available to agent workflows; off avoids that preset. It is not a stronger model or a speed mode.
- **How much:** Experimental and intended for controlled environments. Review exactly which tools/proxy features the selected runtime enables.
- **Speed:** External tools add their own network/process latency; enabling tools is not an inference acceleration setting.
- **VRAM:** No direct base-model allocation change; called tools can use independent GPU resources.
- **RAM:** Tool processes/containers can consume additional RAM outside the model process.

Sources: [official 1](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/tools/server/README.md), [official 2](https://github.com/ggml-org/llama.cpp/blob/2130ca84764e75c7ae9106cec671caa73ddbe416/common/arg.cpp).
