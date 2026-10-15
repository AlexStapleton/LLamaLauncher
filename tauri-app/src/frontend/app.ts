import snapshot from './server-help.txt?raw';
import { parseHelp, valuesFor, displayValues, buildOverrides, optionGroup } from './option-catalog.js';
import { documentationDate } from './option-guide.js';
import { initializeMcp } from './mcp-ui';
import { invoke } from './ipc';
import {connectionAddress} from './listen-address';
import { normalizePath as platformPath, resolveModelPath, parentPath, baseName, commandText } from './platform';
import { listen } from '@tauri-apps/api/event';
import { initializeProfiles } from './profile-ui';
import { initializeFeatures } from './features-ui';
import { initializeStorage } from './storage-ui';

import { control as el, node, errorText } from './dom';
import type { Config, ModelInfo, ArgumentOverride, Profile } from './native-types';
type Setting=ArgumentOverride & {enabled:boolean};
type Settings=Record<string,Setting>;
type Option=ReturnType<typeof parseHelp>[number];
interface Legacy {n_ctx?:number;n_batch?:number;max_tokens?:number;temperature?:number;top_p?:number;top_k?:number;presence_penalty?:number;frequency_penalty?:number;repeat_penalty?:number;n_gpu_layers?:number;gpu_offload?:boolean;verbose?:boolean}
type SavedConfig=Partial<Omit<Config,'mcp_profile'>> & Legacy & {mcp_profile?:Partial<Profile>};
let defaults:Config, config:Config, catalog=parseHelp(snapshot),settings:Settings={},models:ModelInfo[]=[],mcpUi:ReturnType<typeof initializeMcp>,profileUi:ReturnType<typeof initializeProfiles>;
let activeTab = 'setup', lastStatus = 'stopped', busy = false, modelRequest = 0, helpRequest = 0, previewRequest = 0;
let previewTimer:ReturnType<typeof setTimeout>|undefined;
let windows = true, polling = false, profileApplying = false;
let searchTimer:ReturnType<typeof setTimeout>|undefined;
let settingsSaveTimer:ReturnType<typeof setTimeout>|undefined,settingsSaveInFlight=false,pendingWorkingSettings:Config|null=null;
const expansion = new Map<string,boolean>();
let aliases = new Map<string,Option>();
const optionsByTab = new Map<string,Option[]>();
let catalogText=snapshot,optionsRevision=0,renderedOptionsKey='';
function indexCatalog() {
  aliases.clear();optionsByTab.clear();
  for(const option of catalog){
    for(const alias of option.aliases)aliases.set(alias,option);
    const home=option.guide.tab;const group=optionsByTab.get(home)||[];group.push(option);optionsByTab.set(home,group);
  }
  optionsRevision++;
}
indexCatalog();
const managed:Record<string,string> = { '--model': 'models', '--host': 'setup', '--port': 'setup', '--models-dir': 'models' };
const secrets = new Set(['--api-key', '--hf-token', '-hft', '--mcp-servers-json', '--mcp-servers-config']);
const titles:Record<string,string> = { setup: 'Launch', models:'Models', context:'Context & cache', chat:'Chat & output', performance: 'Performance', gpu: 'GPU & memory', sampling: 'Sampling', api: 'API & server', advanced: 'Advanced', all: 'Every executable option', logs: 'Server output', mcp:'MCP connections', profiles:'Profile manager' };
const optionNames:Record<string,string> = {'--ctx-size':'Context length','--batch-size':'Prompt batch size','--ubatch-size':'Physical microbatch','--gpu-layers':'GPU layer offload','--n-gpu-layers':'GPU layer offload','--threads':'CPU generation threads','--threads-batch':'CPU prompt threads','--parallel':'Concurrent request slots','--flash-attn':'Flash attention','--cache-type-k':'Key cache precision','--cache-type-v':'Value cache precision','--load-mode':'Model loading mode','--tensor-split':'GPU memory proportions','--main-gpu':'Primary GPU','--reasoning-budget':'Thinking token budget','--chat-template':'Chat template','--n-predict':'Output token limit','--temp':'Temperature','--top-p':'Top P sampling','--min-p':'Min P sampling','--seed':'Random seed','--api-key':'API authentication key','--timeout':'Request timeout','--sleep-idle-seconds':'Idle sleep delay','--cache-ram':'Prompt cache RAM limit'};

function report(message:unknown, error = false) { el('statusBar').textContent = String(message); el('statusBar').classList.toggle('error', error); }
function hydrate(value:SavedConfig, migrate = false) {
  if ((value.schema_version || 1) > 3) throw new Error('This configuration needs a newer version of LLamaLauncher.');
  config = { ...defaults, ...value,mcp_profile:{...defaults.mcp_profile,...value.mcp_profile} };
  config.schema_version = 3;
  const savedProfile=value.mcp_profile,secureProfile=defaults.mcp_profile || {};
  const samePeer=!savedProfile || (savedProfile.transport===secureProfile.transport && (savedProfile.transport==='http' ? savedProfile.url===secureProfile.url : savedProfile.command===secureProfile.command && JSON.stringify(savedProfile.args||[])===JSON.stringify(secureProfile.args||[])));
  config.mcp_profile = {...secureProfile,...(savedProfile || {})};
  if(!samePeer && savedProfile && !('bearer_token' in savedProfile))config.mcp_profile.bearer_token='';
  if(!samePeer && savedProfile && !('env' in savedProfile))config.mcp_profile.env={};
  settings = {};
  for (const option of config.argument_overrides || []) {
    const found = aliases.get(option.flag);
    settings[found?.flag || option.flag] = { ...option, enabled: true };
  }
  if (migrate) {
    const old = { n_ctx: '--ctx-size', n_batch: '--batch-size', max_tokens: '--n-predict', temperature: '--temp', top_p: '--top-p', top_k: '--top-k', presence_penalty: '--presence-penalty', frequency_penalty: '--frequency-penalty', repeat_penalty: '--repeat-penalty' };
    for (const [key, flag] of Object.entries(old)) if (key in value) {
      const found = catalog.find(option => option.aliases.includes(flag));
      const number = Number(value[key as keyof Legacy]);
      if (found && Number.isFinite(number)) settings[found.flag] = { flag, values: [String(Number(number.toFixed(6)))], enabled: true };
    }
    if ('n_gpu_layers' in value) {
      const option = catalog.find(item => item.aliases.includes('--n-gpu-layers'));
      if (option) settings[option.flag] = { flag: option.flag, values: [value.gpu_offload === false ? '0' : value.n_gpu_layers === -1 ? 'all' : String(value.n_gpu_layers)], enabled: true };
    }
    if (value.verbose) settings['--verbose'] = { flag: '--verbose', values: [], enabled: true };
  }
  for (const id of ['llama_cpp_path', 'models_path', 'model_file', 'host', 'port'] as const) el(id).value = String(config[id]);
  el('additional_args').value = JSON.stringify(config.additional_args || [], null, 2);
  mcpUi?.hydrate();
  syncConfig();
}
function syncConfig() {
  optionsRevision++;
  config.argument_overrides = buildOverrides(settings);
  pendingWorkingSettings=structuredClone(config);
  clearTimeout(settingsSaveTimer);saveState('Unsaved working changes.');settingsSaveTimer=setTimeout(saveWorkingSettings,250);
  const model = models.find(item => normalizePath(item.path) === resolvedModelPath());
  el('summaryModel').textContent = model?.name || baseName(config.model_file, windows) || 'No model selected';
  el('summaryModelMeta').textContent = model ? `${fileSize(model.size)} on disk · weights only` : 'Select a GGUF model or configure a remote model source.';
  if(config.router_mode){el('summaryModel').textContent='Model router';el('summaryModelMeta').textContent=config.models_path?`${models.length} discovered models · loaded on demand`:'Cached models · loaded on demand';}
  el('launch_model_name').textContent=el('summaryModel').textContent;
  el('launch_model_note').textContent=config.router_mode?el('summaryModelMeta').textContent:config.model_file||'No model selected. Open Models and choose a GGUF file before launching.';
  const setting = (aliases:string[]) => Object.values(settings).find(item => item.enabled && aliases.includes(item.flag))?.values.join(' ') || 'Server default';
  el('summaryContext').textContent = setting(['--ctx-size', '-c']);
  el('summaryGpu').textContent = setting(['--gpu-layers', '--n-gpu-layers', '-ngl']);
  el('summaryEndpoint').textContent = `${config.host}:${config.port}`;
  el('network_url').textContent=connectionAddress(config);
  const local=config.host.split(',').every(host=>['127.0.0.1','::1'].includes(host.trim()));
  el('listen_local').setAttribute('aria-pressed',String(local));
  el('listen_lan').setAttribute('aria-pressed',String(config.host==='0.0.0.0'));
  el('summaryOverrides').textContent = String(config.argument_overrides.length);
  clearTimeout(previewTimer); previewTimer = setTimeout(updatePreview, 180);
  profileUi?.changed();
}
let settingsSaveTask=Promise.resolve(true);
function saveState(text:string,retry=false) { const state=document.getElementById('settings_save_state');if(state)state.textContent=text;const button=document.getElementById('settings_save_retry');if(button)button.hidden=!retry; }
async function saveWorkingSettings() {
  while(pendingWorkingSettings || settingsSaveInFlight) {
    if(settingsSaveInFlight){if(!await settingsSaveTask)return false;continue;}
    const value=pendingWorkingSettings!;pendingWorkingSettings=null;settingsSaveInFlight=true;saveState('Saving working settings…');
    settingsSaveTask=invoke('working_save',{config:value}).then(()=>{localStorage.removeItem('llama_config');saveState('Working settings saved.');return true;}).catch(error=>{pendingWorkingSettings=pendingWorkingSettings||value;saveState('Working settings were not saved. Retry or recover the saved record.',true);report('Cannot save working settings: '+String(error),true);return false;}).finally(()=>{settingsSaveInFlight=false;});
    if(!await settingsSaveTask)return false;
  } return true;
}
async function updatePreview() {
  const request = ++previewRequest;
  try {
    const args = await invoke('command_preview', { config });
    if (request === previewRequest) el('preview_text').textContent = args[0] ? commandText(args, windows) : 'Select a llama-server executable in Launch.';
  } catch (error) { if (request === previewRequest) el('preview_text').textContent = String(error); }
}
function normalizePath(path:string) { return platformPath(path, windows); }
function resolvedModelPath() { return resolveModelPath(config.model_file, config.models_path, windows); }
function fileSize(bytes:number) { return bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GiB` : `${(bytes / 1024 ** 2).toFixed(1)} MiB`; }
async function refreshModels(force:unknown = false, preserveSelection = false) {
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
    else if ((preserveSelection || force === true || config.model_id) && config.model_file) {
      const missing=node('option','','Saved model not found — choose a replacement explicitly');missing.value=config.model_file;el('model_select').append(missing);
      report('The saved model was not found. Its path was kept; choose a replacement in Models.',true);
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
    if(text===catalogText){el('catalogStatus').textContent=`${catalog.length} option groups read from your installed executable`;return;}
    const parsed = parseHelp(text);
    if (parsed.length < 5) throw new Error('The executable did not return a recognizable option list.');
    catalog = parsed;
    catalogText=text;
    indexCatalog();
    const remapped:Settings = {};
    for (const entry of Object.values(settings)) { const found = aliases.get(entry.flag); remapped[found?.flag || entry.flag] = entry; }
    settings = remapped;
    updateCatalogControls(); renderOptions(); syncConfig();
    el('catalogStatus').textContent = `${catalog.length} option groups read from your installed executable`;
  } catch (error) { if (request === helpRequest) { el('catalogStatus').textContent = `Bundled catalog (${catalog.length} options). Could not read executable help: ${error}`; } }
}
function updateCatalogControls() {
  el('optionCount').textContent = String(catalog.length);
  const selected = el('option_category').value;
  const all = node('option', '', 'All categories'); all.value = '';
  el('option_category').replaceChildren(all);
  for (const category of [...new Set(catalog.map(item => item.category))].sort()) { const option = node('option', '', category); option.value = category; el('option_category').append(option); }
  el('option_category').value = selected;
}
function groupedOptions(shown:Option[],tab:string,search='',category='',changed=false,compact=false) {
  const fragment = document.createDocumentFragment();
  const groups=new Map<string,Option[]>();
  for(const option of shown){const name=optionGroup(option);if(!groups.has(name))groups.set(name,[]);groups.get(name)!.push(option);}
  for(const [name,options] of groups){
    const group=node('details','option-group');
    const groupKey = `${tab}:${name}`;
    group.open=!!search||!!category||changed||(expansion.get(groupKey) ?? (tab==='all'||(compact?['Model sources','Native tools & MCP overrides','Server diagnostics'].includes(name):!['CPU placement & affinity','Scheduling & polling'].includes(name))));
    const title=node('summary','group-title',name);title.append(node('span','group-count',`${options.length} settings`));
    const list=node('div','option-list');
    let built = false;
    const build = () => { if (!built) { for(const option of options)list.append(optionCard(option)); built = true; } };
    if(group.open) build();
    group.addEventListener('toggle', () => { if(!search&&!category&&!changed)expansion.set(groupKey, group.open); if(group.open)build(); });
    group.append(title,list);fragment.append(group);
  }
  return fragment;
}
const referencePanels=new Map<string,{section:HTMLDetailsElement;search:HTMLInputElement;grid:HTMLDivElement}>();
function mountReferenceOptions() {
  for(const [tab,parent,title] of [['models','models-panel','Model sources, adapters & media options'],['mcp','mcp-panel','Advanced native tools & MCP options'],['logs','logs-panel','Native logging options']] as const){
    const section=node('details','card secondary-details');section.id=tab+'_native_options';section.append(node('summary','',title));
    const search=node('input');search.type='search';search.placeholder='Search these options…';search.setAttribute('aria-label',title);
    const grid=node('div','option-grid');section.append(search,grid);el(parent).append(section);referencePanels.set(tab,{section,search,grid});
    let timer:ReturnType<typeof setTimeout>|undefined;
    search.addEventListener('input',()=>{clearTimeout(timer);timer=setTimeout(()=>{if(activeTab===tab)renderOptions();},100);});section.addEventListener('toggle',()=>{if(activeTab===tab)renderOptions();});
  }
}
function renderOptions() {
  for(const [tab,panel] of referencePanels){if(tab!==activeTab&&panel.grid.childElementCount)panel.grid.replaceChildren();}
  const reference=referencePanels.get(activeTab);
  if(reference){
    if(el('option_grid').childElementCount)el('option_grid').replaceChildren();
    const search=reference.search.value.toLowerCase(),key=JSON.stringify([optionsRevision,activeTab,reference.section.open,search]);
    if(key===renderedOptionsKey)return;renderedOptionsKey=key;
    if(!reference.section.open){reference.grid.replaceChildren();return;}
    const shown=(optionsByTab.get(activeTab)||[]).filter(o=>o.searchText.includes(search));
    reference.grid.replaceChildren(shown.length?groupedOptions(shown,activeTab,search,'',false,true):node('p','helper','No matching options for this executable.'));return;
  }
  if(el('options-panel').hidden){if(el('option_grid').childElementCount)el('option_grid').replaceChildren();renderedOptionsKey='';return;}
  const search=el('option_search').value.toLowerCase(),category=el('option_category').value,changed=el('changed_only').checked;
  const key=JSON.stringify([optionsRevision,activeTab,search,category,changed]);if(key===renderedOptionsKey)return;renderedOptionsKey=key;
  const shown=(activeTab==='all'?catalog:optionsByTab.get(activeTab)||[]).filter(option=>(!category||option.category===category)&&(!changed||settings[option.flag]?.enabled)&&option.searchText.includes(search));
  el('option_grid').replaceChildren(groupedOptions(shown,activeTab,search,category,changed));
  el('filteredCount').textContent=`${shown.length} options`;el('empty_options').hidden=shown.length!==0;
}
function optionCard(option:Option) {
  const setting = settings[option.flag];
  const guide = option.guide;
  const removed=/argument has been removed|DEPRECATED/i.test(option.description);
  const reference=guide.category==='Utilities'||removed;
  const card = node('section', `option-card${setting?.enabled ? ' enabled' : ''}`);
  const heading = node('div', 'option-title');
  const label = guide.reviewed?guide.title:optionNames[option.flag]||option.flag;
  const text = node('div'); text.append(node('h3', '', label), node('p', 'option-category', `${option.flag} · ${option.category}`)); heading.append(text);
  const destination = option.aliases.map(alias => managed[alias]).find(Boolean);
  const enabled = node('input'); enabled.type = 'checkbox'; enabled.checked = !!setting?.enabled;
  enabled.setAttribute('aria-label', `Override ${option.flag}`);
  const check = node('label', 'check-label'); check.append(enabled, document.createTextNode('Override'));
  if (!destination&&!reference) heading.append(check);
  card.append(heading, node('p', 'option-description', guide.summary));
  if(reference) {
    card.append(node('p','helper',removed?'Removed/deprecated in this executable. Leave unset and migrate old overrides using its help.':'Information command: normally prints then exits. Not a serving override.'));
    if(setting?.enabled){const remove=node('button','text-button','Remove saved override');remove.addEventListener('click',()=>{setting.enabled=false;syncConfig();renderOptions();});card.append(remove);}
  }
  else if (destination) {
    const link = node('button', 'text-button', `Configure in ${titles[destination]}`); link.addEventListener('click', () => switchTab(destination)); card.append(link);
    if(setting?.enabled){const remove=node('button','text-button','Remove saved override');remove.addEventListener('click',()=>{setting.enabled=false;syncConfig();renderOptions();});card.append(remove,node('p','helper','An older saved raw override is still enabled. Remove it to use the managed field.'));}
  } else {
    const controls = node('div', 'option-controls');
    const aliases = option.aliases.filter(alias => alias.startsWith('--'));
    const alias = node('select', 'alias-selector'); alias.setAttribute('aria-label', `Flag form for ${option.flag}`);
    for (const flag of aliases.length ? aliases : option.aliases) { const item = node('option', '', flag); item.value = flag; alias.append(item); }
    alias.value = setting?.flag || option.flag; if (!alias.value) alias.value = option.flag;
    alias.hidden=!aliases.some(flag=>flag.startsWith('--no-'));
    let input:HTMLInputElement|HTMLTextAreaElement|HTMLSelectElement|HTMLSpanElement;
    if (!option.syntax) { input = node('span', 'helper', 'Flag only — no value needed.'); }
    else if (option.choices.length) {
      const select = node('select', 'option-value'); input = select;
      const blank = node('option', '', `Select ${option.syntax}`); blank.value = ''; input.append(blank);
      for (const choice of option.choices) { const item = node('option', '', choice); item.value = choice; input.append(item); }
      select.value = setting?.values.join(' ') || (option.choices.includes(option.defaultValue.replace(/^'|'$/g, '')) ? option.defaultValue.replace(/^'|'$/g, '') : '');
      if (setting?.values.length && !select.value) { const item = node('option', '', setting.values.join(' ')); item.value = setting.values.join(' '); select.append(item); select.value = item.value; }
    } else {
      const multiline = /JSON|GRAMMAR|TEMPLATE|MESSAGE|STRING/.test(option.syntax) && !/FILE/.test(option.syntax);
      const field = multiline ? node('textarea','option-value') : node('input','option-value'); input = field;
      if (input instanceof HTMLTextAreaElement) input.rows = 2; else if(input instanceof HTMLInputElement) input.type = option.aliases.some(flag => secrets.has(flag)) ? 'password' : 'text';
      input.spellcheck = false;
      field.value = setting ? displayValues(option, setting.values) : ''; field.placeholder = option.defaultValue ? `Server default: ${option.defaultValue}` : option.syntax;
    }
    if (option.syntax) input.setAttribute('aria-label', `Value for ${option.flag}`);
    controls.append(alias, input);
    const valueInput=input instanceof HTMLSpanElement?undefined:input;
    const save = () => {
      try {
        const values = valueInput ? valuesFor(option, valueInput.value) : [];
        settings[option.flag] = { flag: alias.value, values, enabled: enabled.checked };
        card.classList.toggle('enabled', enabled.checked); if(valueInput)valueInput.disabled = !enabled.checked; alias.disabled = !enabled.checked; syncConfig();
      } catch (error) { report(error, true); }
    };
    enabled.addEventListener('change', () => {
      if(valueInput)valueInput.disabled = !enabled.checked; alias.disabled = !enabled.checked; card.classList.toggle('enabled', enabled.checked);
      if (!enabled.checked) { if (settings[option.flag]) settings[option.flag].enabled = false; syncConfig(); }
      else { if (valueInput && !valueInput.value) { settings[option.flag] = {flag: alias.value, values: [''], enabled: true}; syncConfig(); report(`Enter a value for ${option.flag}.`); input.focus(); } else save(); }
    });
    if (valueInput) valueInput.addEventListener('input', () => { if (enabled.checked) save(); });
    alias.addEventListener('change', save);
    if(valueInput)valueInput.disabled = !enabled.checked; alias.disabled = !enabled.checked;
    if (option.syntax && /FNAME|FILE|PATH/.test(option.syntax) && !/URL/.test(option.syntax)) {
      const browse = node('button', '', '…'); browse.title = `Browse for ${option.flag}`; browse.setAttribute('aria-label', `Browse ${option.flag}`);
      browse.addEventListener('click', async () => {
        try { const directory = /dir|path$/.test(option.flag) && !/file/.test(option.flag); const selected = await invoke('browse_path', { kind: directory ? 'models' : 'file', directory: config.models_path || null }); if (selected && valueInput) { valueInput.value = selected; enabled.checked = true; save(); } }
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
    const list = node('dl');
    for (const [name, effect] of [['WHAT IT DOES',guide.summary],['CHANGE',guide.change],['HOW MUCH',guide.scale],['SPEED',guide.speed],['VRAM',guide.vram],['RAM',guide.ram]])list.append(node('dt','',name),node('dd','',effect));
    details.append(list,node('p','full-help',`Executable help (authoritative for this version): ${option.description}`));
    const provenance=node('p','helper',guide.reviewed?`Guide reviewed against official llama.cpp sources, ${documentationDate}. Defaults/syntax above come from your executable.`:'Version-specific option: no reviewed resource estimate.');
    details.append(provenance);
    if(option.aliases.some(a=>a.startsWith('--no-'))&&option.aliases.some(a=>a.startsWith('--')&&!a.startsWith('--no-')))details.append(node('p','helper','For paired flags, the --no- form selects the disabled behavior described above.'));
    for(const url of guide.sources){const source=node('button','text-button',`Official source · ${url.split('/').slice(-2).join('/')}`);source.type='button';source.addEventListener('click',()=>invoke('open_llama_documentation',{url}).catch(error=>report(error,true)));details.append(source);}

  });
  card.append(details);
  return card;
}
const pageForTab = (tab:string) => ['performance','gpu','context','sampling','chat','api','advanced','all'].includes(tab) ? 'settings' : ['dashboard','test','logs','downloads'].includes(tab) ? 'activity' : tab === 'mcp' ? 'connections' : tab;
function organizeNavigation() {
  const groups:Record<string,string[]> = { settings:['performance','gpu','context','sampling','chat','api','advanced','all'], activity:['dashboard','test','logs','downloads'] };
  const nav = el('subnav');
  const buttons = [...document.querySelectorAll<HTMLButtonElement>('.tab-btn')];
  for (const button of buttons) {
    const group=Object.keys(groups).find(g=>groups[g].includes(button.dataset.tab||'setup'));
    if (group) { button.dataset.group=group; nav.append(button); }
    else button.remove();
  }
  document.querySelectorAll<HTMLButtonElement>('.page-btn').forEach(button=>button.addEventListener('click',()=>{
    const page=button.dataset.page||'setup';
    switchTab(page==='settings'?'performance':page==='activity'?'dashboard':page==='connections'?'mcp':page);
  }));
  switchTab('setup');
}
function switchTab(tab:string) {
  clearTimeout(searchTimer);
  if (tab === 'runtimes') tab='setup';
  activeTab = tab;
  const page=pageForTab(tab);
  document.querySelectorAll<HTMLElement>('.page-btn').forEach(button=>{const active=button.dataset.page===page;button.classList.toggle('active',active);button.setAttribute('aria-current',active?'page':'false');});
  el('subnav').hidden = !['settings','activity'].includes(page);
  document.querySelectorAll<HTMLElement>('.tab-btn').forEach(button=>{button.hidden=button.dataset.group!==page;});
  document.querySelectorAll<HTMLElement>('.tab-btn').forEach(button => { const active = button.dataset.tab === tab; button.classList.toggle('active', active); button.setAttribute('aria-current', active ? 'page' : 'false'); });
  for(const name of ['dashboard','models','test','downloads']) el(`${name}-panel`).hidden=tab!==name;
  el('setup-panel').hidden = tab !== 'setup'; el('logs-panel').hidden = tab !== 'logs'; el('mcp-panel').hidden=tab!=='mcp'; el('profiles-panel').hidden=tab!=='profiles'; el('options-panel').hidden = page!=='settings';
  el('extra-section').hidden = !['advanced', 'all'].includes(tab);
  el('sectionTitle').textContent = titles[tab] || ''; el('option_category').value = ''; el('option_search').value = '';
  const introductions:Record<string,string>={
    performance:'CPU threads, prompt batches and request concurrency. More workers help only until your hardware is saturated.',
    gpu:'GPU weight layers move model weights and computation to GPU. KV cache stores conversation state; its precision and placement are in Context & cache.',
    context:'Context is token capacity, not GPU layer count. More context usually needs more KV cache; lower cache precision saves memory with possible quality and speed tradeoffs.',
    sampling:'Choose how the next token is selected. These mainly change output behavior; they rarely give a predictable inference speedup.',
    chat:'Answer length, chat templates, reasoning and structured output. Longer prompts and answers mean more tokens to process.',
    api:'API access, authentication, endpoints and request handling. The listen address and port are in Launch.',
    advanced:'Speculative decoding keeps target and draft controls together. Unknown runtime options retain their original executable help.',
    all:'Every option reported by your executable, including references that print information and exit. Each setting also has a dedicated home.',
  };
  el('sectionDescription').textContent=(introductions[tab]||'')+' Unchanged options use the executable’s own defaults. Open Help & resource impact for direction, examples and official sources.';
  document.querySelector<HTMLElement>('.settings-panel')!.scrollTop=0;
  renderOptions();if (tab === 'logs') refreshLogs();
}
function showStatus(status:string) {
  const running = status === 'running';
  const alive = ['running', 'loading', 'starting', 'stopping'].includes(status);
  const benchmarking=status==='benchmarking';
  el('serverStatusIndicator').textContent = busy ? 'Working…' : benchmarking ? 'Benchmarking…' : running ? 'Ready' : status === 'loading' ? 'Loading…' : status.startsWith('failed') ? 'Failed' : 'Stopped';
  el('serverStatusIndicator').className = `status-pill ${running ? 'running' : 'stopped'}`;
  el('start_server').disabled = busy || alive || benchmarking || profileApplying; el('stop_server').disabled = busy || !alive;
  profileUi?.controls();
}
function onClick(id:string, action:()=>unknown) { el(id).addEventListener('click', async () => { try { await action(); } catch (error) { report(errorText(error), true); } }); }
async function refreshLogs() { try { const text = await invoke('server_logs'); if(el('log_output').textContent !== text)el('log_output').textContent = text; } catch (error) { el('log_output').textContent = String(error); } }
async function showLaunchFailure(status:string) {
  if(!status.startsWith('failed'))return;
  el('launch_error').hidden=false;el('launch_error').textContent=`Server ${status}. Open Activity → Logs for details.`;
  await refreshLogs();
  let text=el('log_output').textContent;
  for(const value of [config.mcp_profile?.bearer_token,...config.argument_overrides.filter(a=>secrets.has(a.flag)).flatMap(a=>a.values)])if(value)text=text.split(value).join('[redacted]');
  el('launch_error').textContent=`Server ${status}\n${text.split('\n').slice(-12).join('\n')}`;
  report(`Server ${status}. Details are shown in Launch and Activity → Logs.`,true);
}
function snapshotSettings():Config {
  if(el('args_error').textContent)throw new Error('Fix the additional arguments before saving or launching.');
  mcpUi?.snapshot();
  syncConfig();
  return config;
}
async function applyProfile(value:Config) {
  const panel=document.querySelector<HTMLElement>('.settings-panel')!;panel.inert=true;
  profileApplying=true;showStatus(lastStatus);
  try {
    await mcpUi.prepareProfile();
    hydrate(value);await refreshCatalog();await refreshModels(false,true);renderOptions();
  } finally {panel.inert=false;profileApplying=false;showStatus(lastStatus);}
}
async function startServer() {
  if(busy||profileApplying||['running','loading','starting','stopping','benchmarking'].includes(lastStatus))throw new Error('Stop the running server before starting another launch.');
  snapshotSettings();
  el('launch_error').hidden=true;
  if(!config.llama_cpp_path){switchTab('setup');throw new Error('Download llama.cpp in Launch, or browse to an existing llama-server executable.');}
  if(!config.router_mode && !config.model_file && ![...config.argument_overrides.map(a=>a.flag),...config.additional_args.map(a=>a.split('=')[0])].some(flag=>['--hf-repo','-hf','-hfr','--model-url','-mu','--docker-repo','-dr'].includes(flag))){switchTab('models');throw new Error('Choose a GGUF model in Models before starting the server.');}
  for(const setting of Object.values(settings).filter(item=>item.enabled)) {
    const option=aliases.get(setting.flag);if(option?.syntax)valuesFor(option,displayValues(option,setting.values));
  }
  if(!el('port').checkValidity()){switchTab('setup');el('port').reportValidity();throw new Error('Choose a port between 1024 and 65535.');}
  busy=true;showStatus(lastStatus);report('Starting server…');
  try{report('Checking runtime, model and port…');const preflight=await invoke('launch_preflight',{config});const warnings=preflight.warnings||[];report(await invoke('start_server',{config}));if(warnings.length)report(`Server started with ${warnings.length} preflight notes. Review them in Dashboard.`);lastStatus='loading';}finally{busy=false;showStatus(lastStatus);}
}
async function initialize() {
  try { windows = (await invoke('platform_info')).windows; } catch { windows = /Win/.test(navigator.platform); }
  defaults = await invoke('get_defaults');
  let saved:SavedConfig = {}; try { saved = JSON.parse(localStorage.getItem('llama_config') || '{}'); } catch { /* Defaults recover invalid local settings. */ }
  let portableSettings;try{portableSettings=await invoke('working_load');}catch(error){report(error,true);}
  if(portableSettings)saved=portableSettings;
  hydrate(saved, !Array.isArray(saved.argument_overrides)); updateCatalogControls();
  let theme=localStorage.getItem('llama_theme')==='dark'?'dark':'light';
  const setTheme=()=>{document.documentElement.dataset.theme=theme;el('theme_toggle').textContent=theme==='dark'?'Light mode':'Dark mode';el('theme_toggle').setAttribute('aria-pressed',String(theme==='dark'));localStorage.setItem('llama_theme',theme);};
  setTheme();el('theme_toggle').addEventListener('click',()=>{theme=theme==='dark'?'light':'dark';setTheme();});
  mcpUi=initializeMcp({el,node,invoke,getConfig:()=>config,syncConfig,report});mcpUi.hydrate();
  profileUi=initializeProfiles({getConfig:()=>config,snapshot:snapshotSettings,apply:applyProfile,launch:startServer,activeServer:()=>busy||['running','loading','starting','stopping','benchmarking'].includes(lastStatus),open:switchTab,report});
  await initializeStorage(report);
  el('settings_save_retry').addEventListener('click',()=>saveWorkingSettings());
  await listen('launcher-close-request',async()=>{clearTimeout(settingsSaveTimer);const saved=await saveWorkingSettings();if(saved||window.confirm('Working settings could not be saved. Close anyway?'))await invoke('finish_close');}).then(()=>invoke('close_ready')).catch(error=>report(error,true));
  await initializeFeatures({getConfig:()=>config,snapshot:snapshotSettings,apply:applyProfile,report,open:switchTab});
  mountReferenceOptions();
  organizeNavigation();
  document.querySelectorAll<HTMLElement>('.tab-btn').forEach(button => button.addEventListener('click', () => switchTab(button.dataset.tab||'setup')));
  for (const id of ['llama_cpp_path', 'models_path', 'model_file', 'host', 'port'] as const) {
    el(id).addEventListener('input', () => { if(id==='port')config.port=Number(el(id).value);else config[id]=el(id).value; if(id==='llama_cpp_path')config.runtime_id='';if(id==='model_file')config.model_id=''; syncConfig(); if (id === 'model_file') modelDetails(); });
  }
  el('models_path').addEventListener('change', refreshModels); el('llama_cpp_path').addEventListener('change', ()=>refreshCatalog());
  for(const [id,address] of [['listen_local','127.0.0.1'],['listen_lan','0.0.0.0']] as const)onClick(id,()=>{
    el('host').value=address;el('host').dispatchEvent(new Event('input',{bubbles:true}));
    report(address==='0.0.0.0'?'Local network selected. Restart the server to apply; clients use this computer’s local IP.':'Only this computer selected. Restart the server to apply.');
  });
  el('model_select').addEventListener('change', () => { config.model_id='';config.model_file = el('model_select').value; el('model_file').value = config.model_file; modelDetails(); syncConfig(); });
  for (const [id, kind, key] of [['browse_llama_cpp', 'binary', 'llama_cpp_path'], ['browse_models', 'models', 'models_path'], ['browse_model', 'model', 'model_file']] as const) onClick(id, async () => {
    const path = await invoke('browse_path', { kind, directory: config.models_path || null }); if (!path) return;
    config[key] = path;if(key==='llama_cpp_path')config.runtime_id='';if(key==='model_file')config.model_id='';el(key).value = path;
    if (key === 'model_file' && (!config.models_path || !normalizePath(path).startsWith(`${normalizePath(config.models_path).replace(/\/$/, '')}/`))) { config.models_path = parentPath(path, windows); el('models_path').value = config.models_path; }
    syncConfig(); if (key === 'models_path' || key === 'model_file') await refreshModels(); if (key === 'llama_cpp_path') await refreshCatalog(); report('Selection updated.');
  });
  onClick('refresh_models', () => refreshModels(true)); onClick('refresh_options', () => refreshCatalog(true)); onClick('refresh_logs', refreshLogs);
  onClick('save_config', async () => { const path = await invoke('save_config', { config }); if (path) report(`Configuration saved: ${path}`); });
  onClick('load_config', async () => { const loaded = await invoke('load_config'); if (loaded) { await applyProfile(loaded); report('Configuration loaded.'); } });
  onClick('reset_all', async () => { await applyProfile(defaults); report('Server defaults restored.'); });
  onClick('copy_preview', async () => { await navigator.clipboard.writeText(el('preview_text').textContent||''); report('Launch command copied.'); });
  for (const id of ['option_search', 'option_category', 'changed_only'] as const) el(id).addEventListener(id === 'option_search' ? 'input' : 'change', () => { clearTimeout(searchTimer); searchTimer = setTimeout(() => { renderOptions(); document.querySelector<HTMLElement>('.settings-panel')!.scrollTop = 0; }, id === 'option_search' ? 100 : 0); });
  el('additional_args').addEventListener('input', () => {
    try { const args = JSON.parse(el('additional_args').value || '[]'); if (!Array.isArray(args) || args.some(arg => typeof arg !== 'string')) throw new Error('Use a JSON array of strings.'); config.additional_args = args; el('args_error').textContent = ''; syncConfig(); }
    catch (error) { el('args_error').textContent = errorText(error); }
  });
  onClick('choose_launch_model',()=>switchTab('models'));
  onClick('use_single_model',()=>{if(!config.model_file)throw new Error('Choose a model from the list first.');config.router_mode=false;config.mcp_profile.enabled=false;mcpUi.hydrate();syncConfig();switchTab('setup');report('Selected model ready for a single-model launch. MCP is disabled for this launch.');});
  onClick('start_server', async()=>{try{await startServer();}catch(error){el('launch_error').textContent=String(error);el('launch_error').hidden=false;throw error;}});
  onClick('stop_server', async () => { busy = true; showStatus(lastStatus); try { await invoke('stop_server'); lastStatus = 'stopped'; report('Server stopped.'); await refreshLogs(); } finally { busy = false; showStatus(lastStatus); } });
  await listen<string>('server-state', event => { lastStatus = event.payload; showStatus(lastStatus);showLaunchFailure(lastStatus).catch(error=>report(error,true)); }).catch(() => {});
  await listen<{directory:string;visited:number;models:number}>('model-scan', event => { if(event.payload.directory === config.models_path)el('modelCount').textContent=`Scanned ${event.payload.visited.toLocaleString()} entries · ${event.payload.models} GGUF models`; }).catch(()=>{});
  setInterval(async () => {
    if (busy || polling || document.hidden) return;
    polling = true;
    try { const status = await invoke('server_status'); if (lastStatus === 'running' && status !== 'running') { report(`Server ${status}. Open Logs to see why.`, true); await refreshLogs(); } lastStatus = status; showStatus(status); if (activeTab === 'logs') await refreshLogs(); }
    catch (error) { report(error, true); }
    finally { polling = false; }
  }, 5000);
  document.documentElement.dataset.ready = 'true'; report('Ready. Choose a GGUF in Models, then start the server.');
  showStatus('stopped'); await Promise.all([refreshModels(false,true), refreshCatalog()]);
  await profileUi.initialize().catch(error=>report(`Cannot read profiles: ${error}`,true));
  el('profile_storage').textContent=`Saved beside the launcher: ${await invoke('storage_location')}`;
  if(config.mcp_profile.enabled)mcpUi.connect();
}
initialize().catch(error => report(`Cannot initialize: ${error?.message || error}`, true));
