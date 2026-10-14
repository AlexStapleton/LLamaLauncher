import snapshot from './server-help.txt?raw';
import { parseHelp, valuesFor, displayValues, buildOverrides, optionGroup } from './option-catalog.js';
import { impactFor } from './option-impact.js';
import { initializeMcp } from './mcp-ui.js';
import { invoke } from './ipc';
import { normalizePath as platformPath, resolveModelPath, parentPath, baseName, commandText } from './platform';
import { listen } from '@tauri-apps/api/event';
import { initializeProfiles } from './profile-ui';

const el = id => document.getElementById(id);
const node = (tag, className, text) => { const value = document.createElement(tag); if (className) value.className = className; if (text !== undefined) value.textContent = text; return value; };
let defaults, config, catalog = parseHelp(snapshot), settings = {}, models = [], mcpUi, profileUi;
let activeTab = 'setup', lastStatus = 'stopped', busy = false, modelRequest = 0, helpRequest = 0, previewRequest = 0, previewTimer;
let windows = true, polling = false, searchTimer, profileApplying = false;
const expansion = new Map();
let aliases = new Map();
function indexCatalog() { aliases = new Map(catalog.flatMap(option => option.aliases.map(alias => [alias, option]))); }
indexCatalog();
const managed = { '--model': 'setup', '--host': 'api', '--port': 'api' };
const secrets = new Set(['--api-key', '--hf-token', '-hft']);
const titles = { setup: 'Setup', performance: 'Performance', gpu: 'GPU & memory', sampling: 'Sampling', api: 'API & server', advanced: 'Advanced', all: 'Every executable option', logs: 'Server output', mcp:'MCP connections', profiles:'Profile manager' };
const optionNames = {'--ctx-size':'Context length','--batch-size':'Prompt batch size','--ubatch-size':'Physical microbatch','--gpu-layers':'GPU layer offload','--n-gpu-layers':'GPU layer offload','--threads':'CPU generation threads','--threads-batch':'CPU prompt threads','--parallel':'Concurrent request slots','--flash-attn':'Flash attention','--cache-type-k':'Key cache precision','--cache-type-v':'Value cache precision','--load-mode':'Model loading mode','--tensor-split':'GPU memory proportions','--main-gpu':'Primary GPU','--reasoning-budget':'Thinking token budget','--chat-template':'Chat template','--n-predict':'Output token limit','--temp':'Temperature','--top-p':'Top P sampling','--min-p':'Min P sampling','--seed':'Random seed','--api-key':'API authentication key','--timeout':'Request timeout','--sleep-idle-seconds':'Idle sleep delay','--cache-ram':'Prompt cache RAM limit'};

function report(message, error = false) { el('statusBar').textContent = String(message); el('statusBar').classList.toggle('error', error); }
function hydrate(value, migrate = false) {
  if ((value.schema_version || 1) > 2) throw new Error('This configuration needs a newer version of LLamaLauncher.');
  config = { ...defaults, ...value };
  config.schema_version = 2;
  const savedProfile=value.mcp_profile,secureProfile=defaults.mcp_profile || {};
  const samePeer=!savedProfile || (savedProfile.transport===secureProfile.transport && (savedProfile.transport==='http' ? savedProfile.url===secureProfile.url : savedProfile.command===secureProfile.command && JSON.stringify(savedProfile.args||[])===JSON.stringify(secureProfile.args||[])));
  config.mcp_profile = {...secureProfile,...(savedProfile || {})};
  if(!samePeer && !('bearer_token' in savedProfile))config.mcp_profile.bearer_token='';
  if(!samePeer && !('env' in savedProfile))config.mcp_profile.env={};
  settings = {};
  for (const option of config.argument_overrides || []) {
    const found = aliases.get(option.flag);
    settings[found?.flag || option.flag] = { ...option, enabled: true };
  }
  if (migrate) {
    const old = { n_ctx: '--ctx-size', n_batch: '--batch-size', max_tokens: '--n-predict', temperature: '--temp', top_p: '--top-p', top_k: '--top-k', presence_penalty: '--presence-penalty', frequency_penalty: '--frequency-penalty', repeat_penalty: '--repeat-penalty' };
    for (const [key, flag] of Object.entries(old)) if (key in value) {
      const found = catalog.find(option => option.aliases.includes(flag));
      const number = Number(value[key]);
      if (found && Number.isFinite(number)) settings[found.flag] = { flag, values: [String(Number(number.toFixed(6)))], enabled: true };
    }
    if ('n_gpu_layers' in value) {
      const option = catalog.find(item => item.aliases.includes('--n-gpu-layers'));
      if (option) settings[option.flag] = { flag: option.flag, values: [value.gpu_offload === false ? '0' : value.n_gpu_layers === -1 ? 'all' : String(value.n_gpu_layers)], enabled: true };
    }
    if (value.verbose) settings['--verbose'] = { flag: '--verbose', values: [], enabled: true };
  }
  for (const id of ['llama_cpp_path', 'models_path', 'model_file', 'host', 'port']) el(id).value = config[id];
  el('additional_args').value = JSON.stringify(config.additional_args || [], null, 2);
  mcpUi?.hydrate();
  syncConfig();
}
function syncConfig() {
  config.argument_overrides = buildOverrides(settings);
  const persisted = { ...config, argument_overrides: config.argument_overrides.filter(item => !secrets.has(item.flag)) };
  persisted.mcp_profile={...config.mcp_profile};
  delete persisted.mcp_profile.bearer_token;delete persisted.mcp_profile.env;
  if ((persisted.additional_args || []).some(arg => secrets.has(arg.split('=')[0]))) persisted.additional_args = [];
  localStorage.setItem('llama_config', JSON.stringify(persisted));
  const model = models.find(item => normalizePath(item.path) === resolvedModelPath());
  el('summaryModel').textContent = model?.name || baseName(config.model_file, windows) || 'No model selected';
  el('summaryModelMeta').textContent = model ? `${fileSize(model.size)} on disk · weights only` : 'Select a GGUF model or configure a remote model source.';
  if(config.router_mode){el('summaryModel').textContent='Model router';el('summaryModelMeta').textContent=config.models_path?`${models.length} discovered models · loaded on demand`:'Cached models · loaded on demand';}
  const setting = aliases => Object.values(settings).find(item => item.enabled && aliases.includes(item.flag))?.values.join(' ') || 'Server default';
  el('summaryContext').textContent = setting(['--ctx-size', '-c']);
  el('summaryGpu').textContent = setting(['--gpu-layers', '--n-gpu-layers', '-ngl']);
  el('summaryEndpoint').textContent = `${config.host}:${config.port}`;
  el('summaryOverrides').textContent = config.argument_overrides.length;
  clearTimeout(previewTimer); previewTimer = setTimeout(updatePreview, 180);
  profileUi?.changed();
}
async function updatePreview() {
  const request = ++previewRequest;
  try {
    const args = await invoke('command_preview', { config });
    if (request === previewRequest) el('preview_text').textContent = args[0] ? commandText(args, windows) : 'Select a llama-server executable in Setup.';
  } catch (error) { if (request === previewRequest) el('preview_text').textContent = String(error); }
}
function normalizePath(path) { return platformPath(path, windows); }
function resolvedModelPath() { return resolveModelPath(config.model_file, config.models_path, windows); }
function fileSize(bytes) { return bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GiB` : `${(bytes / 1024 ** 2).toFixed(1)} MiB`; }
async function refreshModels(force = false, preserveSelection = false) {
  const request = ++modelRequest, directory = config.models_path;
  el('model_select').replaceChildren(node('option', '', directory ? 'Scanning GGUF files…' : 'Select a models folder first'));
  if (!directory) { await invoke('cancel_models'); models = []; el('modelCount').textContent = 'Select a folder to find models.'; return; }
  el('modelCount').textContent = 'Scanning folder and subfolders…';
  try {
    const found = await invoke('list_models', { directory, force: force === true });
    if (request !== modelRequest || directory !== config.models_path) return;
    models = found;
    const placeholder = node('option', '', found.length ? 'Select a model…' : 'No GGUF models found'); placeholder.value = '';
    el('model_select').replaceChildren(placeholder);
    for (const model of found) { const option = node('option', '', `${model.name} · ${fileSize(model.size)}`); option.value = model.path; el('model_select').append(option); }
    const selected = found.find(item => normalizePath(item.path) === resolvedModelPath());
    if (selected) config.model_file = selected.path;
    else if ((preserveSelection || force === true) && config.model_file) {
      const missing=node('option','','Saved model not found — choose a replacement explicitly');missing.value=config.model_file;el('model_select').append(missing);
      report('The saved model was not found. Its path was kept; choose a replacement in Setup.',true);
    }
    else if (found.length) config.model_file = found[0].path;
    else config.model_file = '';
    el('model_select').value = config.model_file;
    el('model_file').value = config.model_file;
    el('modelCount').textContent = `${found.length} GGUF ${found.length === 1 ? 'model' : 'models'} found · includes subfolders`;
    modelDetails(); syncConfig();
  } catch (error) {
    if (request !== modelRequest) return;
    models = []; el('model_select').replaceChildren(node('option', '', 'Folder could not be scanned'));
    el('modelCount').textContent = String(error); report(error, true);
  }
}
function modelDetails() {
  const model = models.find(item => normalizePath(item.path) === resolvedModelPath());
  el('modelDetails').textContent = model ? `${model.name} · ${fileSize(model.size)} file size. RAM/VRAM requirements also include KV cache and working buffers; file size is not a complete memory estimate.` : config.model_file ? `Saved model not found in this scan: ${config.model_file}. Its path has been kept; choose a replacement explicitly if it moved.` : 'Pick a GGUF model above. Split models use their first shard; other shards are loaded by llama-server.';
}
async function refreshCatalog(force = false) {
  if (!config.llama_cpp_path) { el('catalogStatus').textContent = `${catalog.length} options from the bundled executable snapshot`; return; }
  const request = ++helpRequest;
  el('catalogStatus').textContent = 'Reading --help from your executable…';
  try {
    const text = await invoke('server_help', { binary: config.llama_cpp_path, force: force === true });
    if (request !== helpRequest) return;
    const parsed = parseHelp(text);
    if (parsed.length < 5) throw new Error('The executable did not return a recognizable option list.');
    catalog = parsed;
    indexCatalog();
    const remapped = {};
    for (const entry of Object.values(settings)) { const found = aliases.get(entry.flag); remapped[found?.flag || entry.flag] = entry; }
    settings = remapped;
    updateCatalogControls(); renderOptions(); syncConfig();
    el('catalogStatus').textContent = `${catalog.length} option groups read from your installed executable`;
  } catch (error) { if (request === helpRequest) { el('catalogStatus').textContent = `Bundled catalog (${catalog.length} options). Could not read executable help: ${error}`; } }
}
function updateCatalogControls() {
  el('optionCount').textContent = catalog.length;
  const selected = el('option_category').value;
  const all = node('option', '', 'All categories'); all.value = '';
  el('option_category').replaceChildren(all);
  for (const category of [...new Set(catalog.map(item => item.category))].sort()) { const option = node('option', '', category); option.value = category; el('option_category').append(option); }
  el('option_category').value = selected;
}
function optionsForTab(option) {
  if (activeTab === 'all') return true;
  if (activeTab === 'gpu') return option.category === 'GPU & memory';
  if (activeTab === 'performance') return option.category === 'Performance' || ['--ctx-size', '--load-mode', '--lazy-mode', '--cache-ram'].includes(option.flag);
  if (activeTab === 'sampling') return option.category === 'Sampling';
  if (activeTab === 'api') return option.category === 'API & server';
  if (activeTab === 'advanced') return !['GPU & memory', 'Performance', 'Sampling', 'API & server'].includes(option.category);
  return false;
}
function renderOptions() {
  const search = el('option_search').value.toLowerCase();
  const category = el('option_category').value;
  const shown = catalog.filter(option => optionsForTab(option) && (!category || option.category === category) && (!el('changed_only').checked || settings[option.flag]?.enabled) && option.searchText.includes(search));
  const fragment = document.createDocumentFragment();
  const groups=new Map();
  for(const option of shown){const name=optionGroup(option,activeTab);if(!groups.has(name))groups.set(name,[]);groups.get(name).push(option);}
  for(const [name,options] of groups){
    const group=node('details','option-group');
    const groupKey = `${activeTab}:${name}`;
    group.open=!!search||!!category||el('changed_only').checked||(expansion.get(groupKey) ?? !(activeTab==='performance'&&['CPU placement & affinity','Scheduling & polling','Diagnostics'].includes(name)));
    const title=node('summary','group-title',name);title.append(node('span','group-count',`${options.length} settings`));
    const list=node('div','option-list');
    let built = false;
    const build = () => { if (!built) { for(const option of options)list.append(optionCard(option)); built = true; } };
    if(group.open) build();
    group.addEventListener('toggle', () => { if(!search&&!category&&!el('changed_only').checked)expansion.set(groupKey, group.open); if(group.open)build(); });
    group.append(title,list);fragment.append(group);
  }
  el('option_grid').replaceChildren(fragment);
  el('filteredCount').textContent = `${shown.length} options`;
  el('empty_options').hidden = shown.length !== 0;
}
function optionCard(option) {
  const setting = settings[option.flag];
  const card = node('section', `option-card${setting?.enabled ? ' enabled' : ''}`);
  const heading = node('div', 'option-title');
  const label = optionNames[option.flag] || option.flag.replace(/^--?/, '').replaceAll('-', ' ').replace(/^./, c => c.toUpperCase());
  const text = node('div'); text.append(node('h3', '', label), node('p', 'option-category', `${option.flag} · ${option.category}`)); heading.append(text);
  const destination = option.aliases.map(alias => managed[alias]).find(Boolean);
  const enabled = node('input'); enabled.type = 'checkbox'; enabled.checked = !!setting?.enabled;
  enabled.setAttribute('aria-label', `Override ${option.flag}`);
  const check = node('label', 'check-label'); check.append(enabled, document.createTextNode('Override'));
  if (!destination) heading.append(check);
  card.append(heading, node('p', 'option-description', option.description.length > 125 ? `${option.description.slice(0, 122)}…` : option.description));
  if (destination) {
    const link = node('button', 'text-button', `Configure in ${titles[destination]}`); link.addEventListener('click', () => switchTab(destination)); card.append(link);
  } else {
    const controls = node('div', 'option-controls');
    const aliases = option.aliases.filter(alias => alias.startsWith('--'));
    const alias = node('select', 'alias-selector'); alias.setAttribute('aria-label', `Flag form for ${option.flag}`);
    for (const flag of aliases.length ? aliases : option.aliases) { const item = node('option', '', flag); item.value = flag; alias.append(item); }
    alias.value = setting?.flag || option.flag; if (!alias.value) alias.value = option.flag;
    alias.hidden=!aliases.some(flag=>flag.startsWith('--no-'));
    let input;
    if (!option.syntax) { input = node('span', 'helper', 'Flag only — no value needed.'); }
    else if (option.choices.length) {
      input = node('select', 'option-value');
      const blank = node('option', '', `Select ${option.syntax}`); blank.value = ''; input.append(blank);
      for (const choice of option.choices) { const item = node('option', '', choice); item.value = choice; input.append(item); }
      input.value = setting?.values.join(' ') || (option.choices.includes(option.defaultValue.replace(/^'|'$/g, '')) ? option.defaultValue.replace(/^'|'$/g, '') : '');
      if (setting?.values.length && !input.value) { const item = node('option', '', setting.values.join(' ')); item.value = setting.values.join(' '); input.append(item); input.value = item.value; }
    } else {
      const multiline = /JSON|GRAMMAR|TEMPLATE|MESSAGE|STRING/.test(option.syntax) && !/FILE/.test(option.syntax);
      input = node(multiline ? 'textarea' : 'input', 'option-value');
      if (multiline) input.rows = 2; else input.type = option.aliases.some(flag => secrets.has(flag)) ? 'password' : 'text';
      input.spellcheck = false;
      input.value = setting ? displayValues(option, setting.values) : ''; input.placeholder = option.defaultValue ? `Server default: ${option.defaultValue}` : option.syntax;
    }
    if (option.syntax) input.setAttribute('aria-label', `Value for ${option.flag}`);
    controls.append(alias, input);
    const save = () => {
      try {
        const values = option.syntax ? valuesFor(option, input.value) : [];
        settings[option.flag] = { flag: alias.value, values, enabled: enabled.checked };
        card.classList.toggle('enabled', enabled.checked); input.disabled = !enabled.checked; alias.disabled = !enabled.checked; syncConfig();
      } catch (error) { report(error, true); }
    };
    enabled.addEventListener('change', () => {
      input.disabled = !enabled.checked; alias.disabled = !enabled.checked; card.classList.toggle('enabled', enabled.checked);
      if (!enabled.checked) { if (settings[option.flag]) settings[option.flag].enabled = false; syncConfig(); }
      else { if (option.syntax && !input.value) { settings[option.flag] = {flag: alias.value, values: [''], enabled: true}; syncConfig(); report(`Enter a value for ${option.flag}.`); input.focus(); } else save(); }
    });
    if (option.syntax) input.addEventListener('input', () => { if (enabled.checked) save(); });
    alias.addEventListener('change', save);
    input.disabled = !enabled.checked; alias.disabled = !enabled.checked;
    if (option.syntax && /FNAME|FILE|PATH/.test(option.syntax) && !/URL/.test(option.syntax)) {
      const browse = node('button', '', '…'); browse.title = `Browse for ${option.flag}`; browse.setAttribute('aria-label', `Browse ${option.flag}`);
      browse.addEventListener('click', async () => {
        try { const directory = /dir|path$/.test(option.flag) && !/file/.test(option.flag); const selected = await invoke('browse_path', { kind: directory ? 'models' : 'file', directory: config.models_path || null }); if (selected) { input.value = selected; enabled.checked = true; save(); } }
        catch (error) { report(error, true); }
      }); controls.append(browse);
    }
    card.append(controls, node('p', 'default-note', `Syntax: ${option.syntax || 'flag'} · Default: ${option.defaultValue || 'see executable description'}`));
  }
  const details = node('details', 'impact-details');
  details.append(node('summary', '', 'Help & resource impact'));
  let helpBuilt = false;
  details.addEventListener('toggle', () => {
    if(!details.open || helpBuilt)return;
    helpBuilt = true;
    const effects = impactFor(option);
    const list = node('dl'); for (const [name, effect] of [['SPEED', effects.speed], ['VRAM', effects.vram], ['RAM', effects.ram]]) list.append(node('dt', '', name), node('dd', '', effect));
    details.append(list, node('p', 'full-help', `Executable help: ${option.description}`));
  });
  card.append(details);
  return card;
}
function switchTab(tab) {
  activeTab = tab;
  document.querySelectorAll('.tab-btn').forEach(button => { const active = button.dataset.tab === tab; button.classList.toggle('active', active); button.setAttribute('aria-current', active ? 'page' : 'false'); });
  el('setup-panel').hidden = tab !== 'setup'; el('logs-panel').hidden = tab !== 'logs'; el('mcp-panel').hidden=tab!=='mcp'; el('profiles-panel').hidden=tab!=='profiles'; el('options-panel').hidden = ['setup', 'logs','mcp','profiles'].includes(tab);
  el('api-fields').hidden = tab !== 'api'; el('extra-section').hidden = !['advanced', 'all'].includes(tab);
  el('sectionTitle').textContent = titles[tab]; el('option_category').value = ''; el('option_search').value = '';
  document.querySelector('.settings-panel').scrollTop = 0;
  if (tab === 'logs') refreshLogs(); else renderOptions();
}
function showStatus(status) {
  const running = status === 'running';
  const alive = ['running', 'loading', 'starting', 'stopping'].includes(status);
  el('serverStatusIndicator').textContent = busy ? 'Working…' : running ? 'Ready' : status === 'loading' ? 'Loading…' : status.startsWith('failed') ? 'Failed' : 'Stopped';
  el('serverStatusIndicator').className = `status-pill ${running ? 'running' : 'stopped'}`;
  el('start_server').disabled = busy || alive || profileApplying; el('stop_server').disabled = busy || !alive;
  profileUi?.controls();
}
function onClick(id, action) { el(id).addEventListener('click', async () => { try { await action(); } catch (error) { report(error?.message || error, true); } }); }
async function refreshLogs() { try { const text = await invoke('server_logs'); if(el('log_output').textContent !== text)el('log_output').textContent = text; } catch (error) { el('log_output').textContent = String(error); } }
function snapshotSettings() {
  if(el('args_error').textContent)throw new Error('Fix the additional arguments before saving or launching.');
  mcpUi?.snapshot();
  syncConfig();
  return config;
}
async function applyProfile(value) {
  profileApplying=true;showStatus(lastStatus);
  try {
    await mcpUi.prepareProfile();
    hydrate(value);await refreshCatalog();await refreshModels(false,true);renderOptions();
  } finally {profileApplying=false;showStatus(lastStatus);}
}
async function startServer() {
  if(busy||profileApplying||['running','loading','starting','stopping'].includes(lastStatus))throw new Error('Stop the running server before starting another launch.');
  snapshotSettings();
  for(const setting of Object.values(settings).filter(item=>item.enabled)) {
    const option=aliases.get(setting.flag);if(option?.syntax)valuesFor(option,displayValues(option,setting.values));
  }
  if(!el('port').checkValidity()){switchTab('api');el('port').reportValidity();throw new Error('Choose a port between 1024 and 65535.');}
  busy=true;showStatus(lastStatus);report('Starting server…');
  try{report(await invoke('start_server',{config}));lastStatus='loading';}finally{busy=false;showStatus(lastStatus);}
}
async function initialize() {
  try { windows = (await invoke('platform_info')).windows; } catch { windows = /Win/.test(navigator.platform); }
  defaults = await invoke('get_defaults');
  let saved = {}; try { saved = JSON.parse(localStorage.getItem('llama_config') || '{}'); } catch { /* Defaults recover invalid local settings. */ }
  hydrate(saved, !Array.isArray(saved.argument_overrides)); updateCatalogControls();
  let theme=localStorage.getItem('llama_theme')==='dark'?'dark':'light';
  const setTheme=()=>{document.documentElement.dataset.theme=theme;el('theme_toggle').textContent=theme==='dark'?'Light mode':'Dark mode';el('theme_toggle').setAttribute('aria-pressed',String(theme==='dark'));localStorage.setItem('llama_theme',theme);};
  setTheme();el('theme_toggle').addEventListener('click',()=>{theme=theme==='dark'?'light':'dark';setTheme();});
  mcpUi=initializeMcp({el,node,invoke,getConfig:()=>config,syncConfig,report});mcpUi.hydrate();
  profileUi=initializeProfiles({getConfig:()=>config,snapshot:snapshotSettings,apply:applyProfile,launch:startServer,activeServer:()=>busy||['running','loading','starting','stopping'].includes(lastStatus),open:switchTab,report});
  document.querySelectorAll('.tab-btn').forEach(button => button.addEventListener('click', () => switchTab(button.dataset.tab)));
  for (const id of ['llama_cpp_path', 'models_path', 'model_file', 'host', 'port']) {
    el(id).addEventListener('input', () => { config[id] = id === 'port' ? Number(el(id).value) : el(id).value; syncConfig(); if (id === 'model_file') modelDetails(); });
  }
  el('models_path').addEventListener('change', refreshModels); el('llama_cpp_path').addEventListener('change', refreshCatalog);
  el('model_select').addEventListener('change', () => { config.model_file = el('model_select').value; el('model_file').value = config.model_file; modelDetails(); syncConfig(); });
  for (const [id, kind, key] of [['browse_llama_cpp', 'binary', 'llama_cpp_path'], ['browse_models', 'models', 'models_path'], ['browse_model', 'model', 'model_file']]) onClick(id, async () => {
    const path = await invoke('browse_path', { kind, directory: config.models_path || null }); if (!path) return;
    config[key] = path; el(key).value = path;
    if (key === 'model_file' && (!config.models_path || !normalizePath(path).startsWith(`${normalizePath(config.models_path).replace(/\/$/, '')}/`))) { config.models_path = parentPath(path, windows); el('models_path').value = config.models_path; }
    syncConfig(); if (key === 'models_path' || key === 'model_file') await refreshModels(); if (key === 'llama_cpp_path') await refreshCatalog(); report('Selection updated.');
  });
  onClick('refresh_models', () => refreshModels(true)); onClick('refresh_options', () => refreshCatalog(true)); onClick('refresh_logs', refreshLogs);
  onClick('save_config', async () => { const path = await invoke('save_config', { config }); if (path) report(`Configuration saved: ${path}`); });
  onClick('load_config', async () => { const loaded = await invoke('load_config'); if (loaded) { hydrate(loaded, !Array.isArray(loaded.argument_overrides)); await refreshModels(); await refreshCatalog(); renderOptions(); report('Configuration loaded.'); } });
  onClick('reset_all', async () => { hydrate(defaults); renderOptions(); await refreshModels(); report('Server defaults restored.'); });
  onClick('copy_preview', async () => { await navigator.clipboard.writeText(el('preview_text').textContent); report('Launch command copied.'); });
  for (const id of ['option_search', 'option_category', 'changed_only']) el(id).addEventListener(id === 'option_search' ? 'input' : 'change', () => { clearTimeout(searchTimer); searchTimer = setTimeout(() => { renderOptions(); document.querySelector('.settings-panel').scrollTop = 0; }, id === 'option_search' ? 100 : 0); });
  el('additional_args').addEventListener('input', () => {
    try { const args = JSON.parse(el('additional_args').value || '[]'); if (!Array.isArray(args) || args.some(arg => typeof arg !== 'string')) throw new Error('Use a JSON array of strings.'); config.additional_args = args; el('args_error').textContent = ''; syncConfig(); }
    catch (error) { el('args_error').textContent = error.message; }
  });
  onClick('start_server', startServer);
  onClick('stop_server', async () => { busy = true; showStatus(lastStatus); try { await invoke('stop_server'); lastStatus = 'stopped'; report('Server stopped.'); await refreshLogs(); } finally { busy = false; showStatus(lastStatus); } });
  await listen('server-state', event => { lastStatus = event.payload; showStatus(lastStatus); }).catch(() => {});
  await listen('model-scan', event => { if(event.payload.directory === config.models_path)el('modelCount').textContent=`Scanned ${event.payload.visited.toLocaleString()} entries · ${event.payload.models} GGUF models`; }).catch(()=>{});
  setInterval(async () => {
    if (busy || polling || document.hidden) return;
    polling = true;
    try { const status = await invoke('server_status'); if (lastStatus === 'running' && status !== 'running') { report(`Server ${status}. Open Logs to see why.`, true); await refreshLogs(); } lastStatus = status; showStatus(status); if (activeTab === 'logs') await refreshLogs(); }
    catch (error) { report(error, true); }
    finally { polling = false; }
  }, 5000);
  document.documentElement.dataset.ready = 'true'; report('Ready. Choose a model in Setup; start and stop controls stay visible above.');
  showStatus('stopped'); await Promise.all([refreshModels(), refreshCatalog()]);
  await profileUi.initialize().catch(error=>report(`Cannot read profiles: ${error}`,true));
  if(config.mcp_profile.enabled)mcpUi.connect();
}
initialize().catch(error => report(`Cannot initialize: ${error?.message || error}`, true));
