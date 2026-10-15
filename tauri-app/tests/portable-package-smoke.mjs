import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import net from 'node:net';import {spawn} from 'node:child_process';import {chromium} from 'playwright';
const executable=process.env.LLAMA_LAUNCHER_EXE;assert.ok(executable,'Supply the packaged executable');const folder=path.dirname(executable);assert.ok(fs.existsSync(path.join(folder,'WebView2','msedgewebview2.exe')));assert.ok(fs.existsSync(path.join(folder,'THIRD_PARTY_NOTICES.txt')));assert.equal(fs.existsSync(path.join(folder,'data')),false,'Use a fresh extracted package');
const socket=net.createServer();await new Promise(r=>socket.listen(0,'127.0.0.1',r));const port=socket.address().port;await new Promise(r=>socket.close(r));
const env={...process.env,LOCALAPPDATA:path.join(folder,'test-legacy-context'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port}`};for(const key of ['LLAMA_LAUNCHER_DATA_DIR','WEBVIEW2_BROWSER_EXECUTABLE_FOLDER','WEBVIEW2_USER_DATA_FOLDER','LLAMA_MCP_PROFILE','HF_TOKEN'])delete env[key];
fs.mkdirSync(env.LOCALAPPDATA,{recursive:true});
// Test-only software rendering for a virtual CI desktop. Production launches
// do not receive these Chromium switches or expose a debugging port.
if(process.env.CI==='true')env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS+=' --disable-gpu --no-sandbox';
const child=spawn(executable,[],{env,windowsHide:true,stdio:['ignore','pipe','pipe']});let browser,spawnError='',nativeOutput='';
child.on('error',error=>{spawnError=error.message;});
for(const stream of [child.stdout,child.stderr])stream?.on('data',bytes=>{nativeOutput=(nativeOutput+bytes.toString('utf8')).slice(-16384);});
async function until(fn){const deadline=Date.now()+60000;let lastError='';while(Date.now()<deadline){if(spawnError||child.exitCode!==null||child.signalCode)break;try{const result=await fn();if(result)return result;}catch(error){lastError=error.message;}await new Promise(r=>setTimeout(r,200));}const diagnostic={pid:child.pid,exitCode:child.exitCode,signal:child.signalCode,spawnError,lastError,nativeOutput,profileCreated:fs.existsSync(path.join(folder,'data','webview')),virtualCi:process.env.CI==='true'};fs.mkdirSync('artifacts',{recursive:true});fs.writeFileSync('artifacts/native-package-startup.json',JSON.stringify(diagnostic,null,2));throw Error('Packaged GUI did not initialize: '+JSON.stringify(diagnostic));}
try{browser=await until(()=>chromium.connectOverCDP(`http://127.0.0.1:${port}`));const page=await until(()=>browser.contexts()[0]?.pages()[0]);await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');const location=await page.evaluate(()=>window.__TAURI_INVOKE__('storage_location'));assert.equal(path.resolve(location),path.join(folder,'data'));assert.ok(fs.existsSync(path.join(location,'webview')));assert.equal(await page.locator('#start_server').isVisible(),true);
  assert.equal(await page.locator('#option_grid .option-card').count(),0,'Fresh Launch does not build hidden settings controls');
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  await page.getByRole('button',{name:'Context & cache',exact:true}).click();
  const cache=page.locator('.option-card').filter({has:page.locator('.option-category').filter({hasText:/^--cache-type-v ·/})});
  await cache.locator('.impact-details summary').click();await cache.locator('.impact-details dd').first().waitFor();
  assert.match(await cache.textContent(),/77% combined/);assert.equal(await cache.locator('.impact-details dd').count(),6);
  const blocked=await page.evaluate(async()=>{try{await window.__TAURI_INVOKE__('open_llama_documentation',{url:'https://example.com/unrelated'});return false;}catch(e){return String(e).includes('Only official llama.cpp');}});assert.equal(blocked,true);
  console.log(JSON.stringify({freshPackagedGui:true,packagedFixedWebView:true,dataBesideExecutable:true,dependencyNotices:true,reviewedSettingsHelp:true,nativeDocumentationValidation:true}));}
finally{await browser?.close().catch(()=>{});child.kill();}
