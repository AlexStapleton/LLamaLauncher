// Native default-location and relocation checks; never opens the user's data.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import {spawn} from 'node:child_process';
import {chromium} from 'playwright';
const root=path.resolve(import.meta.dirname,'..');
const fixture=path.join(root,'.test-fixtures',`portable-${Date.now()}`);
const source=process.env.LLAMA_LAUNCHER_EXE||path.join(root,'target/release/llama-server-configurator.exe');
const legacy=path.join(fixture,'legacy');fs.mkdirSync(legacy,{recursive:true});
let browser,child,page;
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function until(fn){const end=Date.now()+30000;while(Date.now()<end){try{const r=await fn();if(r)return r;}catch{}await delay(200);}throw new Error('Portable app did not become ready');}
async function call(command,args={}){return page.evaluate(({command,args})=>window.__TAURI_INVOKE__(command,args),{command,args});}
async function stop(){await browser?.close().catch(()=>{});child?.kill();await delay(500);browser=null;child=null;}
async function launch(folder){
  fs.mkdirSync(folder,{recursive:true});const exe=path.join(folder,'LLamaLauncher.exe');fs.copyFileSync(source,exe);
  const server=net.createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;await new Promise(r=>server.close(r));
  const env={...process.env,LOCALAPPDATA:legacy,APPDATA:legacy,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port}`};
  delete env.LLAMA_LAUNCHER_DATA_DIR;delete env.WEBVIEW2_USER_DATA_FOLDER;delete env.LLAMA_MCP_PROFILE;delete env.HF_TOKEN;
  if(process.env.LLAMA_FIXED_WEBVIEW)env.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=process.env.LLAMA_FIXED_WEBVIEW;
  child=spawn(exe,[],{env,windowsHide:true,stdio:'ignore'});
  browser=await until(()=>chromium.connectOverCDP(`http://127.0.0.1:${port}`));
  page=await until(()=>browser.contexts()[0]?.pages().find(p=>!p.url().startsWith('devtools:')));
  await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
  assert.equal(await call('storage_location'),path.join(folder,'data'));
  await delay(500);
}
try {
  const first=path.join(fixture,'original'),moved=path.join(fixture,'moved');await launch(first);
  const data=path.join(first,'data');const config=await call('get_defaults');
  const saved={...config,llama_cpp_path:path.join(data,'runtimes/test/llama-server.exe'),models_path:path.join(data,'models'),model_file:path.join(data,'models/test.gguf'),router_mode:false,mcp_profile:{...config.mcp_profile,enabled:false}};
  const profile=await call('profiles_save',{id:null,name:'Portable fixture',config:saved});
  await call('working_save',{config:saved});
  assert.ok(fs.existsSync(path.join(data,'launch-profiles.bin')));assert.ok(fs.existsSync(path.join(data,'webview')));
  await stop();fs.mkdirSync(moved,{recursive:true});
  fs.cpSync(data,path.join(moved,'data'),{recursive:true,filter:p=>path.basename(p)!=='webview'});
  await launch(moved);const loaded=await call('profiles_load',{id:profile.id});
  assert.equal(path.normalize(loaded.config.llama_cpp_path),path.join(moved,'data/runtimes/test/llama-server.exe'));
  assert.equal(path.normalize(loaded.config.model_file),path.join(moved,'data/models/test.gguf'));
  assert.equal((await call('working_load')).model_file,loaded.config.model_file);
  await stop();
  const oldProfiles=path.join(legacy,'LlamaServerConfigurator');fs.mkdirSync(oldProfiles,{recursive:true});
  fs.copyFileSync(path.join(data,'launch-profiles.bin'),path.join(oldProfiles,'launch-profiles.bin'));
  await launch(path.join(fixture,'migration'));
  assert.equal((await call('profiles_list'))[0].name,'Portable fixture');
  assert.ok(fs.existsSync(path.join(oldProfiles,'launch-profiles.bin')));
  console.log(JSON.stringify({defaultDataBesideExe:true,profilesSurviveFolderMove:true,managedPathsRebased:true,workingSettingsRebased:true,webviewInsideData:true,legacyProfilesCopiedWithoutDeletion:true}));
} finally {await stop();}
