// Opt-in native Windows integration. Uses a separate user-data directory and
// public HF files; never reads the user's saved MCP/account credentials.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import {spawn} from 'node:child_process';
import {chromium} from 'playwright';
const root=path.resolve(import.meta.dirname,'..');
const executable=process.env.LLAMA_LAUNCHER_EXE||path.join(root,'desktop-app/llama-server-configurator.exe');
assert.ok(process.env.LLAMA_SERVER_EXE,'Supply the official CPU runtime executable.');
const fixture=path.join(root,'.test-fixtures/native-features');fs.mkdirSync(fixture,{recursive:true});
const models=path.join(fixture,'models');fs.mkdirSync(models,{recursive:true});
async function port(){const server=net.createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const value=server.address().port;await new Promise(r=>server.close(r));return value;}
const debugPort=await port();const env={...process.env,LOCALAPPDATA:fixture,LLAMA_LAUNCHER_DATA_DIR:path.join(fixture,"data"),WEBVIEW2_USER_DATA_FOLDER:path.join(fixture,'webview'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${debugPort}`};
if(process.env.LLAMA_FIXED_WEBVIEW)env.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=process.env.LLAMA_FIXED_WEBVIEW;
delete env.LLAMA_MCP_PROFILE;delete env.HF_TOKEN;
const child=spawn(executable,[],{windowsHide:true,env,stdio:'ignore'});let browser,page;let ownedServer=false;
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function until(fn,timeout=30000){const deadline=Date.now()+timeout;let error;while(Date.now()<deadline){try{const value=await fn();if(value)return value;}catch(e){error=e;}await delay(200);}throw error||new Error('Native check timed out');}
async function invoke(command,args={}){return page.evaluate(({command,args})=>window.__TAURI_INVOKE__(command,args),{command,args});}
async function job(id,timeout=180000){const report=await until(async()=>{const r=(await invoke('jobs_list')).find(j=>j.id===id);return r&&r.phase!=='running'?r:null;},timeout);assert.equal(report.phase,'complete',report.message);return report;}

async function resources(){return new Promise((resolve,reject)=>{const p=spawn('powershell.exe',['-NoProfile','-File',path.join(root,'tests/native-resources.ps1'),'-RootProcessId',String(child.pid)],{windowsHide:true,stdio:['ignore','pipe','pipe']});let out='',error='';p.stdout.on('data',b=>out+=b);p.stderr.on('data',b=>error+=b);p.on('error',reject);p.on('exit',code=>{try{if(code)throw Error(error);resolve(JSON.parse(out));}catch(e){reject(e);}});});}
try {
  browser=await until(()=>chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`));
  page=await until(()=>browser.contexts()[0]?.pages().find(p=>!p.url().startsWith('devtools:')));
  await page.waitForFunction(()=>document.documentElement.dataset.ready==='true',{},{timeout:30000});
  await page.getByRole('button',{name:'Activity',exact:true}).click();
  const hardware=await invoke('hardware_snapshot');assert.ok(hardware.total_ram>0);assert.ok(hardware.logical_cores>0);
  await invoke('monitoring_set',{preferences:{enabled:false,interval_seconds:7}});assert.equal((await invoke('monitoring_get')).enabled,false);
  const idleStart=await resources();await delay(5000);const idleEnd=await resources();
  fs.writeFileSync(path.join(root,'artifacts/native-idle-resources.json'),JSON.stringify({scope:'Windows native launcher plus WebView2 process tree; monitoring disabled; five second warm idle sample',start:idleStart,end:idleEnd,cpuPercentOneCore:(idleEnd.cpuSeconds-idleStart.cpuSeconds)/((idleEnd.timestamp-idleStart.timestamp)/1000)*100},null,2));
  const count=(await invoke('monitoring_history')).length;await delay(1500);assert.equal((await invoke('monitoring_history')).length,count);
  await page.screenshot({path:path.join(root,'artifacts/native-dashboard.png')});
  const search=await invoke('hub_search',{search:'SmolLM2-135M',cursor:''});assert.ok(search.models.some(m=>m.id==='bartowski/SmolLM2-135M-Instruct-GGUF'));
  await page.getByRole('button',{name:'Models',exact:true}).click();
  await page.locator('#hub_search').fill('SmolLM2-135M');await page.getByRole('button',{name:'Search',exact:true}).click();
  const hubRow=page.locator('#hub_results .feature-row').filter({hasText:'bartowski/SmolLM2-135M-Instruct-GGUF'});
  await hubRow.getByRole('button',{name:'View GGUF files',exact:true}).click();
  await page.locator('.hub-files input').first().waitFor();
  assert.equal(await page.locator('#hub_repository_files').isVisible(),true);
  await page.screenshot({path:path.join(root,'artifacts/native-hub-files-redesign.png')});
  const repo='bartowski/SmolLM2-135M-Instruct-GGUF';const info=await invoke('hub_details',{repo});const file=info.files.find(f=>f.rfilename==='SmolLM2-135M-Instruct-Q4_K_M.gguf')||info.files.find(f=>f.rfilename.endsWith('.gguf'));assert.ok(file);
  const id=await invoke('download_start',{repo,root:models,files:[file.rfilename]});const downloaded=await job(id);assert.ok(fs.existsSync(downloaded.result.model));
  const runtime=await invoke('runtime_import',{binary:process.env.LLAMA_SERVER_EXE});assert.ok(runtime.version);
  const config={...(await invoke('get_defaults')),llama_cpp_path:runtime.path,runtime_id:runtime.id,models_path:models,model_file:downloaded.result.model,model_id:downloaded.result.model_id,router_mode:false,host:'127.0.0.1',port:await port(),mcp_profile:{...(await invoke('get_defaults')).mcp_profile,enabled:false},argument_overrides:[{flag:'--ctx-size',values:['512']},{flag:'--threads',values:['4']},{flag:'--metrics',values:[]}],additional_args:[]};
  const metadata=await invoke('model_inspect_config',{config});assert.ok(metadata.parameters>0);assert.ok(metadata.estimate_low>metadata.bytes);
  await invoke('launch_preflight',{config});
  await invoke('working_save',{config});await page.reload();await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
  await page.getByRole('button',{name:'Models',exact:true}).click();
  await page.locator('#model_select option').nth(1).waitFor({state:'attached'});
  const choice=await page.locator('#model_select').evaluate(s=>[...s.options].find(o=>o.textContent.includes('Q4_K_M'))?.value||s.options[1].value);
  await page.locator('#model_select').selectOption(choice);
  await page.getByRole('button',{name:'Use for single-model launch',exact:true}).click();
  assert.match(await page.locator('#launch_model_name').textContent(),/gguf/);
  await page.locator('#start_server').click();ownedServer=true;await until(async()=>await invoke('server_status')==='running');
  await page.screenshot({path:path.join(root,'artifacts/native-selected-model-launch.png')});
  const inference=await job(await invoke('inference_test',{prompt:'Say hello in one short sentence.',model:'',maxTokens:32}));assert.ok(inference.result.text.length>0);assert.ok(inference.result.tokens>0);
  await invoke('monitoring_set',{preferences:{enabled:true,interval_seconds:1}});await until(async()=>{const h=await invoke('monitoring_history');return h.some(s=>s.server_ram>0&&Object.keys(s.metrics||{}).length>0);});
  await invoke('stop_server');ownedServer=false;
  const profile=await invoke('profiles_save',{id:null,name:`Native CPU ${Date.now()}`,config});
  const benchmark=await job(await invoke('benchmark_start',{profiles:[profile.id]}));assert.equal(benchmark.result.results[0].runs.length,3);assert.ok(benchmark.result.results[0].runs[0].tokens>0);assert.equal(await invoke('server_status'),'stopped');
  const releases=await invoke('runtime_releases');assert.ok(releases.releases.some(r=>r.assets.some(a=>a.name.includes('cpu-x64'))));assert.ok(releases.releases.every(r=>r.assets.every(a=>!a.name.startsWith('cudart-'))));
  const release=releases.releases.find(r=>r.assets.some(a=>a.name.includes('cpu-x64')));const asset=release.assets.find(a=>a.name.includes('cpu-x64'));await page.getByRole('button',{name:'Launch',exact:true}).click();
  await page.locator('#engine_download').evaluate(n=>n.open=true);
  const stablePlan=await invoke('runtime_channel_plan',{channel:'stable'});assert.equal(stablePlan.channel,'stable');assert.ok(stablePlan.version.startsWith('v'));assert.ok(stablePlan.tag.startsWith('b'));
  const betaPlan=await invoke('runtime_channel_plan',{channel:'beta'});assert.equal(betaPlan.channel,'beta');
  // Deny registry backup publication after a runtime is staged/published; retry must use its receipt.
  const privateRoot=await invoke('storage_location');const registryPath=path.join(privateRoot,'artifacts.bin');const backupPath=registryPath+'.bak';const keptBackup=backupPath+`.fixture-preserved-${Date.now()}`;
  const originalRegistry=fs.readFileSync(registryPath);const hadBackup=fs.existsSync(backupPath);if(hadBackup)fs.renameSync(backupPath,keptBackup);fs.mkdirSync(backupPath);
  try {
    const deniedId=await invoke('runtime_install_channel',{channel:'stable'});
    const denied=await until(async()=>{const r=(await invoke('jobs_list')).find(j=>j.id===deniedId);return r&&r.phase!=='running'?r:null;},180000);
    assert.equal(denied.phase,'failed');assert.match(denied.message,/Cannot replace private profiles/);assert.deepEqual(fs.readFileSync(registryPath),originalRegistry);
    const folders=fs.readdirSync(path.join(privateRoot,'runtimes'),{withFileTypes:true}).filter(e=>e.isDirectory()&&!e.name.startsWith('.'));
    assert.ok(folders.some(e=>fs.existsSync(path.join(privateRoot,'runtimes',e.name,'install-receipt.bin'))));
  } finally {fs.rmdirSync(backupPath);if(hadBackup)fs.renameSync(keptBackup,backupPath);}
  await page.locator('#engine_channel').selectOption('stable');
  await page.getByRole('button',{name:'Download llama.cpp',exact:true}).click();
  const installed=await until(async()=>{const r=(await invoke('jobs_list')).filter(j=>j.kind==='runtime').at(-1);if(r?.phase==='failed')throw new Error(r.message);return r?.phase==='complete'?r:null;},180000);
  assert.ok(installed.result.artifact.managed);assert.ok(fs.existsSync(installed.result.artifact.path));
  await page.waitForFunction(()=>document.getElementById('engine_install_progress').textContent.includes('selected for the next launch.'));
  assert.equal(await page.locator('#llama_cpp_path').inputValue(),installed.result.artifact.path);
  await page.screenshot({path:path.join(root,'artifacts/native-engine-setup-redesign.png')});
  await page.locator('#start_server').click();ownedServer=true;await until(async()=>await invoke('server_status')==='running');
  const stableInference=await job(await invoke('inference_test',{prompt:'Say hello.',model:'',maxTokens:16}));assert.ok(stableInference.result.tokens>0);
  await page.locator('#stop_server').click();ownedServer=false;await until(async()=>await invoke('server_status')==='stopped');
  const savedLocation=await invoke('storage_location');assert.ok(savedLocation.endsWith('data'));
  assert.ok(fs.existsSync(path.join(savedLocation,'working-settings.bin')));
  assert.ok(fs.existsSync(path.join(savedLocation,'launch-profiles.bin')));
  assert.ok(fs.existsSync(path.join(savedLocation,'logs','server-stderr.log')));
  // Relink working settings before removing the temporary managed install.
  await page.locator('#llama_cpp_path').fill(runtime.path);
  await invoke('runtime_remove',{id:installed.result.artifact.id});assert.equal(fs.existsSync(installed.result.artifact.path),false);
  // Two models establish that worker memory is counted, rather than only the router PID.
  const routerFolder=path.join(fixture,'router-models');fs.mkdirSync(routerFolder,{recursive:true});
  for(const name of ['first.gguf','second.gguf']){if(!fs.existsSync(path.join(routerFolder,name)))fs.copyFileSync(config.model_file,path.join(routerFolder,name));}
  await invoke('monitoring_set',{preferences:{enabled:true,interval_seconds:1}});
  await invoke('start_server',{config:{...config,router_mode:true,models_path:routerFolder,port:await port()}});ownedServer=true;await until(async()=>await invoke('server_status')==='running');
  const advertised=await invoke('server_models');assert.ok(advertised.data?.length>=2);const ids=advertised.data.slice(0,2).map(m=>m.id);
  for(const model of ids){await invoke('router_model',{model,load:true});await until(async()=>{const r=await invoke('server_models');return r.data.some(m=>m.id===model&&m.status?.value==='loaded');});}
  const sample=await until(async()=>{const h=await invoke('monitoring_history');return h.at(-1)?.server_ram>200*1024**2?h.at(-1):null;});
  const os=await resources();assert.ok(sample.server_ram>os.rootServerWorkingSetBytes*1.5,'Worker memory must exceed the root-only reading');
  fs.writeFileSync(path.join(root,'artifacts/router-memory-accounting.json'),JSON.stringify({loadedModels:ids.length,sample,os},null,2));
  for(const model of ids)await invoke('router_model',{model,load:false});
  await invoke('monitoring_set',{preferences:{enabled:false,interval_seconds:1}});const stoppedCount=(await invoke('monitoring_history')).length;await delay(1500);assert.equal((await invoke('monitoring_history')).length,stoppedCount);
  await invoke('stop_server');ownedServer=false;
  // Short repeated native lifecycle check; this is not a multi-hour soak.
  for(let i=0;i<3;i++){await invoke('start_server',{config:{...config,port:await port()}});ownedServer=true;await until(async()=>await invoke('server_status')==='running');await invoke('stop_server');ownedServer=false;assert.equal(await invoke('server_status'),'stopped');}
  const report={nativeGui:true,hardware:true,monitoringOff:true,publicHubSearch:true,verifiedModelDownload:true,modelMetadata:true,cpuInference:true,metrics:true,benchmarkRuns:3,routerLoadUnload:true,twoRouterModelsAccounted:true,repeatedNativeStartStop:3,runtimeReleases:true,runtimeInstallRemove:true,runtimeReceiptRetry:true,hubFileButton:true,engineSetupButtons:true,stableBetaChannels:true,portableSettings:true,guiSelectedModelLaunch:true,stableInference:true,toolsExecuted:0};
  fs.writeFileSync(path.join(root,'artifacts/native-features.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
} finally {
  if(ownedServer&&page)await invoke('stop_server').catch(()=>{});
  await browser?.close().catch(()=>{});child.kill();
}
