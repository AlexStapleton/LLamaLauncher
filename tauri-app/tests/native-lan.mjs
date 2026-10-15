// Opt-in: opens an ephemeral IPv4 listener briefly and checks it through this
// computer's LAN IP. Does not modify firewall rules or use real saved accounts.
import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import os from 'node:os';import net from 'node:net';import {spawn} from 'node:child_process';import {chromium} from 'playwright';
const root=path.resolve(import.meta.dirname,'..');for(const name of ['LLAMA_LAUNCHER_EXE','LLAMA_SERVER_EXE','LLAMA_MODEL_FILE'])assert.ok(process.env[name],`Supply ${name}`);
const fixture=path.join(root,'.test-fixtures',`lan-${Date.now()}`);fs.mkdirSync(fixture,{recursive:true});
async function freePort(){const s=net.createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const port=s.address().port;await new Promise(r=>s.close(r));return port;}
const debugPort=await freePort();const env={...process.env,LOCALAPPDATA:fixture,LLAMA_LAUNCHER_DATA_DIR:path.join(fixture,'data'),WEBVIEW2_USER_DATA_FOLDER:path.join(fixture,'webview'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${debugPort}`};if(process.env.LLAMA_FIXED_WEBVIEW)env.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=process.env.LLAMA_FIXED_WEBVIEW;delete env.LLAMA_MCP_PROFILE;delete env.HF_TOKEN;
const child=spawn(process.env.LLAMA_LAUNCHER_EXE,[],{env,windowsHide:true,stdio:'ignore'});let browser,page,owned=false;
async function until(fn){const deadline=Date.now()+45000;while(Date.now()<deadline){try{const value=await fn();if(value)return value;}catch{}await new Promise(r=>setTimeout(r,200));}throw Error('Native LAN check timed out');}
async function invoke(command,args={}){return page.evaluate(({command,args})=>window.__TAURI_INVOKE__(command,args),{command,args});}
try{
  browser=await until(()=>chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`));page=await until(()=>browser.contexts()[0]?.pages()[0]);await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
  const port=await freePort();const config={...await invoke('get_defaults'),llama_cpp_path:process.env.LLAMA_SERVER_EXE,models_path:path.dirname(process.env.LLAMA_MODEL_FILE),model_file:process.env.LLAMA_MODEL_FILE,host:'0.0.0.0',port,argument_overrides:[{flag:'--ctx-size',values:['512']},{flag:'--threads',values:['2']}]};
  await invoke('start_server',{config});owned=true;await until(async()=>await invoke('server_status')==='running');
  const addresses=Object.values(os.networkInterfaces()).flat().filter(a=>a&&!a.internal&&a.family==='IPv4').map(a=>a.address);assert.ok(addresses.length,'No non-loopback IPv4 interface');
  for(const address of addresses){const response=await fetch(`http://${address}:${port}/health`,{signal:AbortSignal.timeout(5000)});assert.equal(response.status,200);}
  const address=addresses[0];await invoke('stop_server');owned=false;
  await invoke('start_server',{config:{...config,host:address,port:await freePort()}});owned=true;await until(async()=>await invoke('server_status')==='running');
  const id=await invoke('inference_test',{prompt:'Say hello.',model:'',maxTokens:8});const result=await until(async()=>{const job=(await invoke('jobs_list')).find(j=>j.id===id);return job&&job.phase!=='running'?job:null;});assert.equal(result.phase,'complete',result.message);assert.ok(result.result.tokens>0);
  await invoke('stop_server');owned=false;const report={wildcardBind:true,localIpv4InterfacesChecked:addresses.length,specificInterfaceBind:true,inference:true,firewallChanged:false,remoteDeviceTested:false};fs.writeFileSync(path.join(root,'artifacts/native-lan.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{if(owned&&page)await invoke('stop_server').catch(()=>{});await browser?.close().catch(()=>{});child.kill();}
