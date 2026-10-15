import { invoke } from "./ipc";
import {decodeJobReport} from './job-results';

import { listen as nativeListen, type Event } from "@tauri-apps/api/event";
let eventWarning:(message:string)=>void=()=>{};
const failedListeners=new Set<string>();
async function listen<T>(name: string, handler: (event: Event<T>) => void) {
  return nativeListen<T>(name, handler).catch(() => {failedListeners.add(name);eventWarning(`Live updates unavailable for ${name}. Refresh/polling remains available.`); return () => {};});
}

import type {
  Config,
  JobReport,
  TelemetrySample,
  Artifact,
  ModelMetadata,
} from "./native-types";

interface Context {
  getConfig: () => Config;
  snapshot: () => Config;
  apply: (config: Config) => Promise<void>;
  report: (message: unknown, error?: boolean) => void;
  open: (tab: string) => void;
}

const element = <K extends keyof HTMLElementTagNameMap>(
  tag: K,
  text?: string,
) => {
  const n = document.createElement(tag);
  if (text !== undefined) n.textContent = text;
  return n;
};

const bytes = (n: unknown) =>
  typeof n === "number" ? `${(n / 1073741824).toFixed(2)} GiB` : "Unavailable";

const number = (n: unknown, suffix = "") =>
  typeof n === "number" ? `${n.toFixed(2)}${suffix}` : "Unavailable";

function heading(title: string, description: string) {
  const h = element("div");
  h.className = "page-heading";
  const c = element("div");
  c.append(element("h2", title), element("p", description));
  h.append(c);
  return h;
}

function card(title: string) {
  const c = element("section");
  c.className = "card";
  c.append(element("h3", title));
  return c;
}

function button(text: string, action: () => unknown, ctx: Context) {
  const b = element("button", text);
  b.type = "button";
  b.addEventListener("click", async () => {
    const owner = b.closest(".card");
    const status = owner?.querySelector<HTMLElement>(":scope > .action-feedback");
    b.disabled = true;
    b.setAttribute("aria-busy", "true");
    if (status) { status.textContent = `${text}…`; status.classList.remove("error-text"); }
    try {
      await action();
      if (status) status.textContent = `${text} complete.`;
    } catch (e) {
      if (status) { status.textContent = String(e); status.classList.add("error-text"); }
      ctx.report(String(e), true);
    } finally {
      b.disabled = b.dataset.unavailable === "true";
      b.removeAttribute("aria-busy");
    }
  });
  return b;
}

function field(label: string, input: HTMLElement) {
  const l = element("label", label);
  l.className = "feature-field";
  l.append(input);
  return l;
}

function output() {
  const p = element("pre");
  p.className = "feature-output";
  return p;
}

function json(value: unknown) {
  return JSON.stringify(value, null, 2);
}

function metric(container: HTMLElement, label: string, value: string) {
  const c = element("div");
  c.className = "metric";
  c.append(element("span", label), element("strong", value));
  container.append(c);
}

function section(title: string, label: string) {
  const panel = element("div");
  panel.id = `${title}-panel`;
  panel.className = "page-panel";
  panel.hidden = true;
  document.querySelector(".settings-panel")!.append(panel);
  const tab = element("button", label);
  tab.className = "tab-btn";
  tab.dataset.tab = title;
  document.querySelector(".tabs")!.append(tab);
  return panel;
}

interface HubModel {
  id: string;
  pipeline_tag?: string;
  downloads?: number;
}

interface HubFile {
  rfilename: string;
  size?: number;
  lfs?: { size?: number; sha256?: string };
}

interface HubDetails {
  id: string;
  sha: string;
  files: HubFile[];
  card: Record<string, unknown>;
  card_text?: string;
  gated: unknown;
  pipeline?: string;
}

interface RouterModel {
  id: string;
  status?: unknown;
  name?: string;
}

export async function initializeFeatures(ctx: Context) {
  eventWarning=message=>ctx.report(message,true);
  const dashboard = section("dashboard", "Dashboard"),
    models = section("models", "Models"),
    test = section("test", "Test"),
    runtimes = document.getElementById("setup-panel")!;

  dashboard.append(
    heading(
      "Dashboard",
      "Hardware checks, live resource use, and server metrics.",
    ),
  );

  const controls = card("Monitoring");
  const enabled = element("input");
  enabled.type = "checkbox";
  enabled.id = "monitoring_enabled";
  const interval = element("input");
  interval.type = "number";
  interval.min = "1";
  interval.max = "60";
  interval.id = "monitoring_interval";
  const monitoringStatus = element("p");
  monitoringStatus.className = "helper";
  controls.append(
    field("Enable monitoring", enabled),
    field("Polling interval (seconds)", interval),
    monitoringStatus,
  );

  const prefs = await invoke("monitoring_get").catch(error=>{ctx.report(error,true);return {enabled:false,interval_seconds:2};});
  enabled.checked = prefs.enabled;
  interval.value = String(prefs.interval_seconds);
  let generation = 0;
  let lastValidInterval = prefs.interval_seconds;
  interval.disabled = !enabled.checked;

  async function saveMonitoring() {
    if (enabled.checked && !interval.checkValidity()) {
      interval.reportValidity();
      return;
    }
    if (interval.checkValidity()) lastValidInterval = Number(interval.value);
    else interval.value = String(lastValidInterval);
    interval.disabled = !enabled.checked;
    const g = ++generation;
    try {
      await invoke("monitoring_set", {
        preferences: {
          enabled: enabled.checked,
          interval_seconds: lastValidInterval,
        },
      });
      if (g === generation)
        monitoringStatus.textContent = enabled.checked
          ? `Sampling every ${interval.value} seconds`
          : "Monitoring off — retained readings show their last-update time.";
    } catch (error) {
      if (g === generation) {
        const actual = await invoke("monitoring_get");
        enabled.checked = actual.enabled;
        interval.value = String(actual.interval_seconds);
        interval.disabled = !actual.enabled;
        monitoringStatus.textContent =
          "Preference could not be saved; previous monitoring settings remain active.";
      }
      throw error;
    }
  }

  enabled.addEventListener("change", () =>
    saveMonitoring().catch((e) => ctx.report(e, true)),
  );
  interval.addEventListener("change", () =>
    saveMonitoring().catch((e) => ctx.report(e, true)),
  );
  monitoringStatus.textContent = enabled.checked
    ? `Sampling every ${interval.value} seconds`
    : "Monitoring off";
  dashboard.append(controls);

  const hardware = card("Hardware");
  const hardwareValues = element("div");
  hardwareValues.className = "metric-grid";
  const hardwareNotes = element("p");
  hardwareNotes.className = "helper";
  const hardwareButton = button(
    "Detect hardware",
    async () => {
      const h = await invoke("hardware_snapshot");
      hardwareValues.replaceChildren();
      metric(hardwareValues, "CPU", h.cpu);
      metric(hardwareValues, "Logical cores", String(h.logical_cores));
      metric(hardwareValues, "Installed RAM", bytes(h.total_ram));
      metric(hardwareValues, "Available RAM", bytes(h.available_ram));
      for (const gpu of h.gpus) {
        metric(hardwareValues, String(gpu.name), `${bytes(gpu.total)} VRAM`);
      }
      hardwareNotes.textContent = h.gpus.length
        ? "GPU telemetry is whole-device usage."
        : "Live GPU telemetry unavailable. Read the selected llama.cpp engine’s devices in Launch for backend compatibility.";
    },
    ctx,
  );
  hardware.append(hardwareButton, hardwareValues, hardwareNotes);
  dashboard.append(hardware);

  const live = card("Live resources");
  const readings = element("div");
  readings.className = "metric-grid";
  const timestamp = element("p");
  timestamp.className = "helper";
  const graph = element("canvas");
  graph.width = 700;
  graph.height = 130;
  graph.className = "telemetry-chart";
  graph.setAttribute("aria-label", "Five-minute CPU utilization history");
  const metrics = output();
  const history: TelemetrySample[] = await invoke("monitoring_history").catch(error=>{ctx.report(error,true);return [];});

  function renderSample(s: TelemetrySample) {
    readings.replaceChildren();
    metric(readings, "System CPU", number(s.cpu_percent, "%"));
    metric(
      readings,
      "Server tree CPU (sum of cores)",
      number(s.server_cpu_percent, "%"),
    );
    metric(readings, "Server tree RAM (workers + helpers)", bytes(s.server_ram));
    metric(readings, "Available RAM", bytes(s.available_ram));
    for (const gpu of s.gpus) {
      metric(
        readings,
        String(gpu.name),
        `${bytes(gpu.used)} / ${bytes(gpu.total)} · ${number(gpu.utilization, "%")}`,
      );
    }
    timestamp.textContent = `Last reading: ${new Date(s.timestamp * 1000).toLocaleTimeString()}`;
    metrics.textContent = Object.keys(s.metrics || {}).length
      ? json(s.metrics)
      : "Inference metrics unavailable. Enable --metrics in API settings before starting the server.";
    const c = graph.getContext("2d");
    if (c) {
      c.clearRect(0, 0, graph.width, graph.height);
      c.strokeStyle = "#4c82cd";
      c.lineWidth = 2;
      c.beginPath();
      const last = s.timestamp;
      history.forEach((v, i) => {
        const x = graph.width * (1 - (last - v.timestamp) / 300);
        const y = graph.height * (1 - Math.min(100, v.cpu_percent) / 100);
        if (i) c.lineTo(x, y);
        else c.moveTo(x, y);
      });
      c.stroke();
    }
  }

  live.append(
    readings,
    timestamp,
    graph,
    element("p", "CPU history · last five minutes"),
    metrics,
  );
  dashboard.append(live);
  if (history.length) renderSample(history[history.length - 1]);

  await listen<TelemetrySample>("launcher-telemetry", (e) => {
    if (!enabled.checked) return;
    history.push(e.payload);
    while (
      history.length > 300 ||
      history[0]?.timestamp + 300 < e.payload.timestamp
    )
      history.shift();
    if(!document.hidden&&!dashboard.hidden)renderSample(e.payload);
  });

  const preflight = card("Launch preflight");
  const preflightOutput = output();
  const suggestions=element('div');let checkedConfig='';let suggested:{flag:string;values:string[];reason:string}[]=[];
  const applySuggestions=button('Apply suggested CPU threads',async()=>{if(JSON.stringify(ctx.snapshot())!==checkedConfig)throw new Error('Settings changed. Run preflight again before applying suggestions.');const c=ctx.snapshot();await ctx.apply({...c,argument_overrides:[...c.argument_overrides.filter(a=>!suggested.some(s=>s.flag===a.flag)),...suggested.map(({flag,values})=>({flag,values}))]});ctx.report('Suggested CPU threads applied to working settings. Save your profile to keep them.');},ctx);applySuggestions.disabled=true;
  preflight.append(
    button(
      "Check current launch",
      async () => {
        preflightOutput.textContent = "Checking runtime, model and port…";
        try {
          const config=ctx.snapshot();const result=await invoke("launch_preflight",{config});preflightOutput.textContent=json(result);checkedConfig=JSON.stringify(config);suggested=(result.suggestions||[])as {flag:string;values:string[];reason:string}[];suggestions.replaceChildren(...suggested.map(s=>element('p',`${s.flag} ${s.values.join(' ')} — ${s.reason}`)));applySuggestions.disabled=!suggested.length;
        } catch (e) {
          applySuggestions.disabled=true;preflightOutput.textContent = String(e);
          throw e;
        }
      },
      ctx,
    ),
    suggestions,applySuggestions,preflightOutput,
  );
  dashboard.append(preflight);

  models.append(
    heading(
      "Models",
      "Inspect local models or download GGUF files from Hugging Face.",
    ),
  );

  const local = card("Local model inspector");
  const inspected = element("div");
  const details = output();
  local.append(
    button(
      "Inspect selected model",
      async () => {
        const c = ctx.snapshot();
        const option = (names: string[], fallback: number) =>
          Number(
            c.argument_overrides.find((a) => names.includes(a.flag))
              ?.values[0] || fallback,
          );
        const m: ModelMetadata = await invoke("model_inspect_config", {
          config: c,
        });
        inspected.replaceChildren();
        inspected.className = "metric-grid";
        metric(inspected, "Architecture", m.architecture);
        metric(
          inspected,
          "Parameters",
          m.parameters ? `${(m.parameters / 1e9).toFixed(2)} B` : "Unknown",
        );
        metric(inspected, "Quantization", m.quantization);
        metric(inspected, "Trained context", String(m.context || "Unknown"));
        metric(inspected, "File size", bytes(m.bytes));
        metric(
          inspected,
          "Estimated total memory",
          m.estimate_low
            ? `${bytes(m.estimate_low)} – ${bytes(m.estimate_high)}`
            : "Unknown",
        );
        if (m.resource_estimate) {
          metric(
            inspected,
            "Estimated RAM",
            `${bytes(m.resource_estimate.ram_low)} – ${bytes(m.resource_estimate.ram_high)}`,
          );
          metric(
            inspected,
            "Estimated aggregate VRAM",
            `${bytes(m.resource_estimate.vram_low)} – ${bytes(m.resource_estimate.vram_high)}`,
          );
        }
        details.textContent = `${m.warnings.join("\n")}\n\n${json(m.metadata)}`;
      },
      ctx,
    ),
    inspected,
    details,
  );
  models.append(local);
  const library = card("Downloaded model library");
  const libraryRows = element("div");
  async function refreshModelLibrary() {
    libraryRows.replaceChildren();
    for (const artifact of (await invoke("artifacts_list")).filter(
      (a) => a.kind === "model",
    )) {
      const row = element("div");
      row.className = "feature-row";
      row.append(
        element(
          "strong",
          `${artifact.name} · ${artifact.path.split(/[\\/]/).pop()}`,
        ),
        button(
          "Use local model",
          async () => {
            const c = ctx.snapshot();
            if (
              c.argument_overrides.some((a) =>
                [
                  "--hf-repo",
                  "-hf",
                  "-hfr",
                  "--model-url",
                  "-mu",
                  "--docker-repo",
                  "-dr",
                ].includes(a.flag),
              ) ||
              c.additional_args.some((a) =>
                [
                  "--hf-repo",
                  "-hf",
                  "-hfr",
                  "--model-url",
                  "-mu",
                  "--docker-repo",
                  "-dr",
                ].includes(a.split("=")[0]),
              )
            )
              throw new Error(
                "Disable remote model-source arguments before using a local model.",
              );
            await ctx.apply({
              ...c,
              model_file: artifact.path,
              model_id: artifact.id,
              models_path: String(
                artifact.details.root ||
                  artifact.path.slice(
                    0,
                    Math.max(
                      artifact.path.lastIndexOf("/"),
                      artifact.path.lastIndexOf("\\"),
                    ),
                  ),
              ),
            });
            ctx.open("setup");
          },
          ctx,
        ),
      );
      libraryRows.append(row);
    }
  }
  library.append(
    button("Refresh model library", refreshModelLibrary, ctx),
    libraryRows,
  );
  models.append(library);

  const account = card("Hugging Face account");
  const accountStatus = element("p");
  accountStatus.id = "hub_account_status";
  accountStatus.className = "helper";
  const token = element("input");
  token.type = "password";
  token.autocomplete = "off";
  token.placeholder = "Read token (optional for public models)";
  token.id = "hub_token";
  const clientId = element("input");
  clientId.placeholder = "Public OAuth client ID";
  clientId.id = "hub_client_id";
  let accountConnected = false;

  async function refreshAccount() {
    const a = await invoke("hub_account");
    accountConnected = Boolean(a.connected);
    accountStatus.textContent = a.connected
      ? `Connected as ${a.name}`
      : "Browsing public models without an account";
    clientId.value = String(a.client_id || "");
    browserSignIn.disabled = !clientId.value.trim();
    testAccount.disabled = !accountConnected;
    disconnectAccount.disabled = !accountConnected;
    testAccount.dataset.unavailable = String(!accountConnected);
    disconnectAccount.dataset.unavailable = String(!accountConnected);
  }

  const auth = element("div");
  auth.className = "feature-actions";
  const testAccount = button("Test connection", async () => {
    await invoke("hub_test"); await refreshAccount();
    ctx.report("Hugging Face connection passed.");
  }, ctx);
  const disconnectAccount = button("Disconnect", async () => {
    await invoke("hub_disconnect"); await refreshAccount();
  }, ctx);
  testAccount.disabled = true; disconnectAccount.disabled = true;
  const browserSignIn = button("Sign in with browser", async () => {
    const id = await invoke("hub_sign_in", { clientId: clientId.value.trim() });
    await refreshJobs(); await invoke("hub_open_sign_in", { id });
    ctx.open("downloads");
  }, ctx);
  browserSignIn.disabled = !clientId.value.trim();
  clientId.addEventListener("input", () => { browserSignIn.disabled = !clientId.value.trim(); });
  const browserAuth = element("details");
  browserAuth.className = "secondary-details";
  browserAuth.append(element("summary", "Advanced: browser sign-in"),
    element("p", "Requires a registered public Hugging Face OAuth client ID. Until one is configured, use a read token above. Public searches and downloads work without signing in."),
    field("Public OAuth client ID", clientId), browserSignIn);
  auth.append(
    button(
      "Connect token",
      async () => {
        const value = token.value.trim();
        if (!value) throw new Error("Paste a Hugging Face read token first. Public models do not need an account.");
        token.value = "";
        await invoke("hub_connect", { token: value });
        await refreshAccount();
      },
      ctx,
    ),
    button("Get a read token", () => invoke("hub_open_tokens"), ctx),
    testAccount, disconnectAccount,
  );
  account.append(
    accountStatus,
    field("Read token", token),
    auth,
    element(
      "p",
      "Browser sign-in requires a registered public OAuth application. Tokens stay in native private storage. Gated models may also require license acceptance or access approval on Hugging Face.",
    ),
    browserAuth,
  );
  document.getElementById("mcp-panel")!.prepend(account);

  const searchCard = card("Search Hugging Face");
  const search = element("input");
  search.type = "search";
  search.placeholder = "Model name or author";
  search.id = "hub_search";
  const results = element("div");
  results.id = "hub_results";
  const filesCard = card("Repository files");
  filesCard.id = "hub_repository_files";
  filesCard.hidden = true;
  filesCard.tabIndex = -1;
  const filesList = element("div");
  filesList.className = "hub-files";
  const repoInfo = element("p");
  const cardData = output();
  let selectedRepo: HubDetails | null = null,
    cursor = "";
  const checks = new Map<string, HTMLInputElement>();

  async function selectRepo(repo: string) {
    const request = ++repositoryRequest;
    selectedRepo = null; checks.clear(); filesList.replaceChildren();
    filesCard.hidden = false;
    modelCard.open = false;
    repoInfo.textContent = `Loading GGUF files for ${repo}…`;
    cardData.textContent = "";
    filesCard.scrollIntoView({block:"start", behavior:"smooth"});
    filesCard.focus({preventScroll:true});
    let d: HubDetails;
    try { d = (await invoke("hub_details", { repo })) as unknown as HubDetails; }
    catch(error) { if (request === repositoryRequest) repoInfo.textContent = String(error); throw error; }
    if (request !== repositoryRequest) return;
    selectedRepo = d;
    checks.clear();
    filesList.replaceChildren();
    repoInfo.textContent = `${d.id} · ${d.sha.slice(0, 12)} · ${d.pipeline || "Compatibility unknown"}${d.gated ? " · Gated access" : ""}`;
    cardData.textContent = `${json(d.card || {})}\n\n${d.card_text || "Open the model page to read its full model card."}`;
    for (const f of [...d.files].sort((a,b)=>Number(/(?:^|\/)mmproj/i.test(a.rfilename))-Number(/(?:^|\/)mmproj/i.test(b.rfilename)) || a.rfilename.localeCompare(b.rfilename))) {
      const check = element("input");
      check.type = "checkbox";
      checks.set(f.rfilename, check);
      check.addEventListener("change", () => {
        const split = f.rfilename.match(/^(.*)-\d{5}-of-(\d{5})\.gguf$/);
        if (split)
          for (const [name, c] of checks)
            if (
              name.startsWith(`${split[1]}-`) &&
              name.endsWith(`-of-${split[2]}.gguf`)
            )
              c.checked = check.checked;
      });
      filesList.append(
        field(`${f.rfilename} · ${bytes(f.lfs?.size ?? f.size)}`, check),
      );
    }
    if (!d.files.length)
      filesList.append(element("p", "No GGUF files in this repository."));
  }
  let repositoryRequest = 0;
  let searchRequest = 0;

  async function searchModels(more = false) {
    const request = ++searchRequest;
    moreButton.disabled = true;
    moreButton.dataset.unavailable = "true";
    if (!more) {
      cursor = "";
      results.replaceChildren(element("p", "Searching…"));
    }
    const r = (await invoke("hub_search", {
      search: search.value,
      cursor: more ? cursor : "",
    })) as unknown as { models: HubModel[]; cursor: string };
    if (request !== searchRequest) return;
    if (!more) results.replaceChildren();
    cursor = r.cursor;
    for (const m of r.models) {
      const row = element("div");
      row.className = "feature-row";
      row.append(
        element("strong", m.id),
        element(
          "span",
          `${m.pipeline_tag || "Compatibility unknown"} · ${m.downloads || 0} downloads`,
        ),
        button("View GGUF files", () => selectRepo(m.id), ctx),
      );
      results.append(row);
    }
    moreButton.disabled = !cursor;
    moreButton.dataset.unavailable = String(!cursor);
    if (!r.models.length && !more)
      results.append(
        element(
          "p",
          "No compatible results on this page. Try another search or More results.",
        ),
      );
  }

  const moreButton = button("More results", () => searchModels(true), ctx);
  moreButton.disabled = true;
  searchCard.append(
    search,
    button("Search", () => searchModels(), ctx),
    results,
    moreButton,
  );
  search.addEventListener("keydown", (e) => {
    if (e.key === "Enter") searchModels().catch((e) => ctx.report(e, true));
  });
  models.append(searchCard);
  searchCard.append(element("p", "Public models need no login. For private or gated access, connect your Hugging Face account in Connections."),button("Hugging Face account settings",()=>ctx.open("mcp"),ctx));

  filesCard.append(
    repoInfo,
    filesList,
    button(
      "Open model page / request access",
      async () => {
        if (selectedRepo)
          await invoke("open_hub_page", { repo: selectedRepo.id });
      },
      ctx,
    ),
    button(
      "Download selected files",
      async () => {
        if (!selectedRepo) throw new Error("Choose a repository first.");
        const c = ctx.snapshot();
        if (!c.models_path)
          throw new Error("Select a models folder in Models first.");
        const files = [...checks].filter(([, c]) => c.checked).map(([n]) => n);
        await invoke("download_start", {
          repo: selectedRepo.id,
          root: c.models_path,
          files,
        });
        await refreshJobs();
        await refreshDownloads();
        ctx.open("downloads");
      },
      ctx,
    ),
    cardData,
  );
  models.append(filesCard);

  const downloadCard = card("Downloads");
  const downloadsPanel = section("downloads", "Downloads & tasks");
  downloadsPanel.append(heading("Downloads & tasks", "Model downloads, llama.cpp installation, sign-in, and benchmark progress."));
  const downloadList = element("div");
  const jobList = element("div");
  jobList.id = "launcher_jobs";
  downloadCard.append(
    button(
      "Refresh downloads",
      async () => {
        await refreshJobs();
        await refreshDownloads();
      },
      ctx,
    ),
    downloadList,
    jobList,
  );
  downloadsPanel.append(downloadCard);

  let inferenceId = "";
  let benchmarkResult: Record<string, unknown> | null = null;
  const jobReports = new Map<string, JobReport>();

  const jobRows=new Map<string,{node:HTMLDivElement;signature:string}>();
  function renderJobs() {
    if(document.hidden||downloadsPanel.hidden)return;
    for(const [id,row] of jobRows){if(!jobReports.has(id)){row.node.remove();jobRows.delete(id);}}
    for (const report of [...jobReports.values()].slice(-64).reverse()) {
      const signature=JSON.stringify({kind:report.kind,phase:report.phase,result:report.phase==='running'&&report.kind!=='sign-in'?null:report.result});
      const previous=jobRows.get(report.id);
      if(previous?.signature===signature){
        previous.node.querySelector('p')!.textContent=report.message;
        let progress=previous.node.querySelector('progress');let label=previous.node.querySelector<HTMLElement>('.job-progress-label');
        if(report.total&&!progress){progress=element('progress');label=element('span');label.className='job-progress-label';previous.node.append(progress,label);}
        if(progress){progress.max=Math.max(1,report.total);progress.value=report.completed;}
        if(label)label.textContent=report.kind==='benchmark'?report.completed+' / '+report.total+' profiles':bytes(report.completed)+' / '+bytes(report.total);
        continue;
      }
      const row = element("div");row.dataset.jobId=report.id;
      row.className = "feature-job";
      row.append(
        element("strong", `${report.kind} · ${report.phase}`),
        element("p", report.message),
      );
      if (report.total) {
        const progress = element("progress");
        progress.max = report.total;
        progress.value = report.completed;
        row.append(
          progress,
          element(
            "span",
            report.kind === "benchmark"
              ? `${report.completed} / ${report.total} profiles`
              : `${bytes(report.completed)} / ${bytes(report.total)}`,
          ),
        );
      }
      if (report.phase === "running")
        row.append(
          button(
            report.kind === "download" ? "Pause" : "Cancel",
            () => invoke("job_cancel", { id: report.id }),
            ctx,
          ),
        );
      if (report.kind === "sign-in" && report.phase === "running") {
        row.append(
          element("p", `Authorization code: ${report.result?.code || ""}`),
          button(
            "Open authorization page",
            () => invoke("hub_open_sign_in", { id: report.id }),
            ctx,
          ),
        );
      }
      if (report.kind === "download" && report.phase === "complete") {
        const variants = report.result.models || [];
        const variant = element("select");
        variants.forEach((m, i) => {
          const o = element("option", String(m.name || m.model));
          o.value = String(i);
          variant.append(o);
        });
        if (variants.length > 1) row.append(field("Downloaded model", variant));
        row.append(
          button(
            "Use downloaded model",
            () => useDownloaded(report, Number(variant.value) || 0),
            ctx,
          ),
          button(
            "Create profile",
            async () => {
              await useDownloaded(report, Number(variant.value) || 0);
              ctx.open("profiles");
              ctx.report(
                "Downloaded model selected. Enter a custom profile name and save current settings.",
              );
            },
            ctx,
          ),
        );
      }
      for(const label of row.querySelectorAll(":scope > span"))label.classList.add("job-progress-label");
      if(previous)previous.node.replaceWith(row);else if(jobRows.size)jobList.prepend(row);else jobList.append(row);
      jobRows.set(report.id,{node:row,signature});
    }
  }

  async function useDownloaded(report: JobReport, index = 0) {
    const variants = report.result.models || [];
    const value = { ...report.result, ...(variants[index] || {}) };
    const c = ctx.snapshot();
    const source = [
      "--hf-repo",
      "-hf",
      "-hfr",
      "--model-url",
      "-mu",
      "--docker-repo",
      "-dr",
      "--hf-file",
      "-hff",
      "--model",
      "-m",
    ];
    if (
      c.argument_overrides.some((o) => source.includes(o.flag)) ||
      c.additional_args.some((a) => source.includes(a.split("=")[0]))
    ) {
      throw new Error(
        "Disable remote/custom model-source arguments before selecting a downloaded local model.",
      );
    }
    await ctx.apply({
      ...c,
      model_file: String(value.model),
      models_path: String(value.directory),
      model_id: String(value.model_id),
    });
    ctx.open("setup");
  }

  async function refreshJobs() {
    for (const raw of await invoke("jobs_list")) {
      const report=decodeJobReport(raw);
      jobReports.set(report.id, report);
      updateJobResult(report);
    }
    renderJobs();
  }

  async function refreshDownloads() {
    downloadList.replaceChildren();
    for (const plan of await invoke("downloads_list")) {
      const row = element("div");
      row.className = "feature-row";
      row.append(
        element("strong", `${plan.repo} · ${plan.revision.slice(0, 12)}`),
        element("span", `${plan.files.length} files`),
        button(
          "Resume / retry",
          async () => {
            await invoke("download_resume", { id: plan.id });
            await refreshJobs();
            await refreshDownloads();
          },
          ctx,
        ),
        button(
          "Remove partial files",
          async () => {
            if (
              !window.confirm(
                "Remove resumable partial files? Completed model files will be kept.",
              )
            )
              return;
            await invoke("download_remove", { id: plan.id });
            await refreshDownloads();
          },
          ctx,
        ),
      );
      downloadList.append(row);
    }
  }

  test.append(
    heading(
      "Inference test",
      "Send one streaming prompt to the active server. No tools are executed and prompts are not saved.",
    ),
  );
  const testCard = card("Prompt");
  const prompt = element("textarea");
  prompt.rows = 4;
  prompt.id = "test_prompt";
  prompt.placeholder = "Ask a short question…";
  const testModel = element("select");
  testModel.id = "test_model";
  testModel.append(element("option", "Server default"));
  const max = element("input");
  max.type = "number";
  max.min = "1";
  max.max = "4096";
  max.value = "128";
  const response = output();
  response.id = "test_response";
  const timing = element("p");
  timing.className = "helper";
  testCard.append(
    field("Model", testModel),
    button(
      "Refresh active models",
      async () => {
        const r = await invoke("server_models");
        const list = (r.data || r.models || []) as RouterModel[];
        testModel.replaceChildren();
        for (const m of list) {
          const opt = element("option", m.id);
          opt.value = m.id;
          testModel.append(opt);
        }
        if (!list.length) {
          const opt = element("option", "Server default");
          opt.value = "";
          testModel.append(opt);
        }
      },
      ctx,
    ),
    field("Prompt", prompt),
    field("Maximum output tokens", max),
    button(
      "Send test",
      async () => {
        if (inferenceId && jobReports.get(inferenceId)?.phase === "running")
          throw new Error("Wait for the current test or cancel it.");
        response.textContent = "";
        timing.textContent = "Waiting for first token…";
        inferenceId = await invoke("inference_test", {
          prompt: prompt.value,
          model: testModel.value === "Server default" ? "" : testModel.value,
          maxTokens: Number(max.value),
        });
        await refreshJobs();
      },
      ctx,
    ),
    button(
      "Cancel test",
      async () => {
        if (inferenceId) await invoke("job_cancel", { id: inferenceId });
      },
      ctx,
    ),
    button("Open server chat", () => invoke("open_server_chat"), ctx),
    timing,
    response,
  );
  test.append(testCard);

  const router = card("Router models");
  const routerRows = element("div");
  async function refreshRouter() {
    const r = await invoke("server_models");
    routerRows.replaceChildren();
    const list = (r.data || r.models || []) as RouterModel[];
    for (const m of list) {
      const row = element("div");
      row.className = "feature-row";
      row.append(
        element("strong", m.id),
        element(
          "span",
          typeof m.status === "string"
            ? m.status
            : json(m.status || "Advertised"),
        ),
        button(
          "Load",
          async () => {
            await invoke("router_model", { model: m.id, load: true });
            await refreshRouter();
          },
          ctx,
        ),
        button(
          "Unload",
          async () => {
            await invoke("router_model", { model: m.id, load: false });
            await refreshRouter();
          },
          ctx,
        ),
      );
      routerRows.append(row);
    }
    if (!list.length)
      routerRows.append(
        element("p", "No models advertised by the active server."),
      );
  }
  router.append(button("Refresh router state", refreshRouter, ctx), routerRows);
  models.append(router);

  const installed = card("Installed llama.cpp engines");
  installed.id = "installed_engines";
  const installedRows = element("div");
  async function refreshRuntimes() {
    installedRows.replaceChildren();
    for (const a of (await invoke("artifacts_list")).filter(
      (a) => a.kind === "runtime",
    )) {
      const row = element("div");
      row.className = "feature-job";
      row.append(
        element("strong", a.version),
        element("p", a.path),
        element("p", String(a.details.devices || "")),
        button(
          "Use this runtime",
          async () => {
            await ctx.apply({
              ...ctx.snapshot(),
              llama_cpp_path: a.path,
              runtime_id: a.id,
            });
            ctx.report(
              "Runtime selected for the next launch. Save your profile to keep this change.",
            );
          },
          ctx,
        ),
        button(
          "Remove runtime",
          async () => {
            if (
              !window.confirm(
                "Remove this runtime? Referenced profiles must be relinked first.",
              )
            )
              return;
            await invoke("runtime_remove", { id: a.id });
            await refreshRuntimes();
          },
          ctx,
        ),
      );
      if (a.path === ctx.getConfig().llama_cpp_path) row.prepend(element("span", "Selected for next launch"));
      installedRows.append(row);
    }
    if (!installedRows.childElementCount) installedRows.append(element("p", "No engines registered yet. Download an official release below, or browse to your existing llama-server executable above and import it."));
  }
  installed.append(
    button(
      "Import selected executable",
      async () => {
        await invoke("runtime_import", {
          binary: ctx.snapshot().llama_cpp_path,
        });
        await refreshRuntimes();
      },
      ctx,
    ),
    button("Refresh installed", refreshRuntimes, ctx),
    installedRows,
  );
  const releases = card("Official releases");
  releases.id = "engine_setup";
  releases.append(element("p", "Install llama.cpp directly from its official GitHub releases, or use your existing copy. The launcher and engine update independently; no extra CPU copy is required."));
  const releaseSelect = element("select");
  releaseSelect.id="engine_channel";
  for (const [value,label] of [["stable","Stable"],["beta","Beta (nightly)"]]) {
    const option=element("option",label);option.value=value;releaseSelect.append(option);
  }
  const releaseSummary=element("p");releaseSummary.className="helper";
  releaseSummary.textContent="Stable follows the latest official stable release. Beta follows the newest development build. The launcher selects the Windows/Linux x64 build for your hardware automatically.";
  let setupInstallId="", setupInstallBasePath="";
  const appliedInstalls=new Set<string>();
  const setupProgress=element("div");setupProgress.id="engine_install_progress";
  const installEngine=button("Download llama.cpp",async()=>{
    setupInstallBasePath=ctx.getConfig().llama_cpp_path;
    releaseSummary.textContent="Finding the official release and detecting a compatible build…";
    setupInstallId=await invoke("runtime_install_channel",{channel:releaseSelect.value});
    await refreshJobs();
    ctx.report("Downloading llama.cpp from its official GitHub release. Progress appears in Launch and Activity.");
  },ctx);
  installEngine.classList.add("primary");
  releases.replaceChildren(element("h3","Download llama.cpp"),
    field("Release channel",releaseSelect),installEngine,releaseSummary,setupProgress);
  const executableCard = document.getElementById("llama_cpp_path")!.closest(".card")!;
  const engineDownload = element("details"); engineDownload.className="engine-details"; engineDownload.id="engine_download";
  engineDownload.open=!ctx.getConfig().llama_cpp_path;
  engineDownload.append(element("summary", "Download or update llama.cpp from GitHub"), releases);
  const engineLibrary = element("details"); engineLibrary.className="engine-details"; engineLibrary.id="engine_library";
  engineLibrary.append(element("summary", "Manage installed llama.cpp engines"),installed);
  executableCard.append(engineDownload, engineLibrary);
  runtimes.querySelector(".page-heading")!.after(executableCard);
  const launchSummary=document.getElementById("launch_model_name")!.closest(".card")!;
  const modeCard=runtimes.querySelector(".mode-card")!;
  const modeFields=element("div");modeFields.className="launch-mode";
  modeFields.append(...modeCard.childNodes);launchSummary.append(modeFields);modeCard.remove();

  const benchmark = card("Benchmark profiles");
  const benchmarkProfiles = element("div");
  const benchmarkOutput = output();
  const benchmarkSummary = element("div");
  const benchmarkHistory = element("select");
  benchmarkHistory.setAttribute("aria-label", "Saved benchmark results");
  benchmarkOutput.id = "benchmark_results";
  const selectedProfiles = new Map<string, HTMLInputElement>();
  async function refreshBenchmarkProfiles() {
    benchmarkProfiles.replaceChildren();
    selectedProfiles.clear();
    for (const p of await invoke("profiles_list")) {
      const check = element("input");
      check.type = "checkbox";
      selectedProfiles.set(p.id, check);
      benchmarkProfiles.append(field(p.name, check));
    }
  }
  benchmark.append(
    element(
      "p",
      "Stop the server first. Each profile uses one warm-up and three measured runs. Benchmarks collect temporary measurements even when dashboard monitoring is off. Tools and MCP are disabled; router profiles use their selected local model in single-model mode.",
    ),
    button("Refresh profiles", refreshBenchmarkProfiles, ctx),
    benchmarkProfiles,
    button(
      "Compare selected profiles",
      async () => {
        await invoke("benchmark_start", {
          profiles: [...selectedProfiles]
            .filter(([, c]) => c.checked)
            .map(([id]) => id),
        });
        await refreshJobs();
        ctx.open("downloads");
        ctx.report(
          "Benchmark started. Progress and cancellation are in Activity → Downloads & tasks.",
        );
      },
      ctx,
    ),
    button(
      "Export JSON",
      async () => {
        if (benchmarkResult)
          await invoke("results_export", {
            result: benchmarkResult,
            csv: false,
          });
      },
      ctx,
    ),
    button(
      "Export CSV",
      async () => {
        if (benchmarkResult)
          await invoke("results_export", {
            result: benchmarkResult,
            csv: true,
          });
      },
      ctx,
    ),
    button(
      "Load saved results",
      async () => {
        const saved = await invoke("results_list");
        benchmarkHistory.replaceChildren();
        saved.forEach((r, i) => {
          const o = element(
            "option",
            new Date(Number(r.timestamp) * 1000).toLocaleString(),
          );
          o.value = String(i);
          benchmarkHistory.append(o);
        });
        benchmarkHistory.onchange = () =>
          showBenchmark(saved[Number(benchmarkHistory.value)]);
        if (saved.length) showBenchmark(saved[0]);
      },
      ctx,
    ),
    benchmarkHistory,
    benchmarkSummary,
    benchmarkOutput,
  );
  document.getElementById("profiles-panel")!.append(benchmark);

  await listen<{ id: string; text: string }>("inference-delta", (e) => {
    if (e.payload.id === inferenceId) response.textContent += e.payload.text;
  });

  function showBenchmark(result: Record<string, unknown>) {
    benchmarkResult = result;
    benchmarkOutput.textContent = json(result);
    benchmarkSummary.replaceChildren();
    const table = element("table");
    table.className = "benchmark-table";
    const head = element("tr");
    for (const h of [
      "Profile",
      "First token (s)",
      "Tokens/s",
      "Startup (s)",
      "Sampled peak RAM",
    ])
      head.append(element("th", h));
    table.append(head);
    for (const row of (result.results || []) as Record<string, unknown>[]) {
      const runs = (row.runs || []) as Record<string, unknown>[];
      const mean = (key: string) => {
        const values = runs
          .map((r) => r[key])
          .filter((v): v is number => typeof v === "number");
        return values.length
          ? values.reduce((a, b) => a + b, 0) / values.length
          : null;
      };
      const tr = element("tr");
      for (const value of [
        String(row.profile),
        number(mean("first_token_seconds")),
        number(mean("tokens_per_second")),
        number(row.startup_seconds),
        bytes(row.peak_server_ram),
      ])
        tr.append(element("td", value));
      table.append(tr);
    }
    benchmarkSummary.append(table);
  }

  const terminalResults=new Map<string,string>();
  function updateJobResult(r: JobReport) {
    if(r.phase!=='running'){
      const key=`${r.id}|${setupInstallId}|${inferenceId}`;const signature=JSON.stringify({phase:r.phase,result:r.result});
      if(terminalResults.get(key)===signature)return;terminalResults.set(key,signature);
      while(terminalResults.size>128)terminalResults.delete(terminalResults.keys().next().value!);
    }
    if (r.kind === "runtime" && r.id === setupInstallId) {
      setupProgress.replaceChildren(element("p", `${r.phase} · ${r.message}`));
      if (r.phase === "running") setupProgress.append(button("Cancel installation",()=>invoke("job_cancel",{id:r.id}),ctx));
      const selection=r.result.selection;
      if(selection)releaseSummary.textContent=`${selection.label} ${selection.version} · ${selection.tag} · ${selection.backend}. ${selection.reason}`;
      if (r.phase === "complete" && !appliedInstalls.has(r.id)) {
        appliedInstalls.add(r.id);
        const use=async()=>{
          const artifact=r.result.artifact;if(!artifact)throw Error('Completed install is missing its artifact');
          await ctx.apply({...ctx.snapshot(),llama_cpp_path:artifact.path,runtime_id:artifact.id});
          await refreshRuntimes();engineLibrary.open=false;engineDownload.open=false;
          setupProgress.replaceChildren(element("p","llama.cpp installed and selected for the next launch."));
          ctx.report("llama.cpp is ready. Choose a model in Models, then start the server.");
        };
        if(ctx.getConfig().llama_cpp_path===setupInstallBasePath)use().catch(e=>ctx.report(e,true));
        else setupProgress.append(button("Use installed llama.cpp",use,ctx),element("p","Your executable selection changed during the download, so it has been kept."));
      }
    }
    if (
      r.kind === "inference" &&
      r.id === inferenceId &&
      r.phase !== "running"
    ) {
      if (r.phase === "complete") {
        response.textContent = String(r.result.text || "");
        timing.textContent = `First token: ${number(r.result.first_token_seconds, " s")} · ${number(r.result.tokens_per_second, " tokens/s")} · ${number(r.result.seconds, " s total")}`;
      } else timing.textContent = r.message;
    }
    if (r.kind === "sign-in" && r.phase === "complete")
      refreshAccount().catch((e) => ctx.report(e, true));
    if (r.kind === "download" && r.phase === "complete")
      refreshModelLibrary().catch((e) => ctx.report(e, true));
    if (r.kind === "runtime" && r.phase === "complete")
      refreshRuntimes().catch((e) => ctx.report(e, true));
    if (r.kind === "benchmark" && r.phase === "complete")
      showBenchmark(r.result);
  }

  await listen<JobReport>("launcher-job", (e) => {
    try{e.payload=decodeJobReport(e.payload);}catch(error){ctx.report(error,true);return;}
    jobReports.set(e.payload.id, e.payload);
    if (jobReports.size > 64) {
      const old = [...jobReports].find(([, r]) => r.phase !== "running");
      if (old) jobReports.delete(old[0]);
    }
    renderJobs();
    updateJobResult(e.payload);
  });

  await Promise.allSettled([
    refreshAccount(),
    refreshRuntimes(),
    refreshDownloads(),
    refreshJobs(),
    refreshBenchmarkProfiles(),
    refreshModelLibrary(),
  ]);

  let routerEvents = false;
  let routerRefresh: ReturnType<typeof setTimeout> | undefined;
  await listen<{ connected: boolean }>("router-events", (e) => {
    routerEvents = e.payload.connected;
  });
  await listen("router-change", () => {
    clearTimeout(routerRefresh);
    routerRefresh = setTimeout(() => {
      if (!models.hidden) refreshRouter().catch((e) => ctx.report(e, true));
    }, 300);
  });
  // Router state uses a separate lifecycle poll, independent of hardware monitoring.

  let routerPolling = false;
  let fallbackBusy=false,lastTelemetryPoll=0;
  setInterval(async()=>{
    if(document.hidden||fallbackBusy)return;
    try{fallbackBusy=true;
      if(failedListeners.has('launcher-job'))await refreshJobs();
      if(failedListeners.has('launcher-telemetry')&&enabled.checked&&!dashboard.hidden&&Date.now()-lastTelemetryPoll>=Number(interval.value)*1000){
        lastTelemetryPoll=Date.now();const saved=await invoke('monitoring_history');history.splice(0,history.length,...saved);if(history.length)renderSample(history[history.length-1]);
      }
    }catch(error){ctx.report(error,true);}finally{fallbackBusy=false;}
  },1000);
  setInterval(async () => {
    if (
      models.hidden ||
      document.hidden ||
      routerPolling ||
      routerEvents ||
      !ctx.getConfig().router_mode
    )
      return;
    routerPolling = true;
    try {
      await refreshRouter();
    } catch {
      /* An idle/stopped router is displayed by explicit refresh. */
    } finally {
      routerPolling = false;
    }
  }, 5000);
  // Search opens directly into a nearby file picker; local management follows it.
  const folderCard=document.getElementById("models_path")!.closest(".card")!;
  models.replaceChildren(models.firstElementChild!,folderCard,library,searchCard,filesCard,local,router);
  const guide=document.querySelector<HTMLElement>(".intro-card")!;
  const guideDetails=element("details");guideDetails.className="secondary-details";
  guideDetails.append(element("summary","Speed and memory guide"),guide);
  document.querySelector(".option-toolbar")!.before(guideDetails);
  const searchButton = [...searchCard.querySelectorAll("button")].find(b=>b.textContent==="Search")!;
  const searchBar=element("div"); searchBar.className="path-row";
  searchCard.insertBefore(searchBar,search); searchBar.append(search,searchButton);
  const modelCard = element("details"); modelCard.className="secondary-details";
  modelCard.append(element("summary", "Model card and metadata"),cardData);
  modelCard.addEventListener("toggle", async()=>{
    const repo=selectedRepo;
    if (!modelCard.open || !repo || repo.card_text !== undefined) return;
    repo.card_text = "";
    cardData.textContent = `${json(repo.card || {})}\n\nLoading model card…`;
    try {
      const text=await invoke("hub_read_card",{repo:repo.id,revision:repo.sha});
      if (repo===selectedRepo) cardData.textContent=`${json(repo.card || {})}\n\n${text || "Open the model page to read its full model card."}`;
    } catch(error) { if(repo===selectedRepo) cardData.textContent=`${json(repo.card || {})}\n\n${String(error)}`; }
  });
  filesCard.append(modelCard);
  const refreshVisible=()=>{if(!document.hidden){if(!dashboard.hidden&&history.length)renderSample(history[history.length-1]);if(!downloadsPanel.hidden)renderJobs();}};
  const visibilityObserver=new MutationObserver(refreshVisible);
  visibilityObserver.observe(dashboard,{attributes:true,attributeFilter:['hidden']});visibilityObserver.observe(downloadsPanel,{attributes:true,attributeFilter:['hidden']});
  document.addEventListener('visibilitychange',refreshVisible);
  // Cards use consistent action bars, with feedback next to the initiating action.
  document.querySelectorAll<HTMLElement>(".card").forEach(c => {
    const direct = [...c.children]; let actions: HTMLElement | undefined;
    for (const n of direct) {
      if (n instanceof HTMLButtonElement) {
        if (!actions) { actions=element("div"); actions.className="feature-actions"; c.insertBefore(actions,n); }
        actions.append(n);
      } else actions=undefined;
    }
    if (!c.querySelector(":scope > .action-feedback")) {
      const feedback=element("p"); feedback.className="action-feedback"; feedback.setAttribute("role","status"); feedback.setAttribute("aria-live","polite"); c.append(feedback);
    }
  });
}
