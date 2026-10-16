import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {chromium} from 'playwright';
const executable=process.env.LLAMA_LAUNCHER_EXE;
assert.ok(executable,'Supply the packaged executable');
const folder=path.dirname(executable),location=path.join(folder,'data'),virtualCi=process.env.CI==='true';
assert.ok(fs.existsSync(path.join(folder,'WebView2','msedgewebview2.exe')));
assert.ok(fs.existsSync(path.join(folder,'THIRD_PARTY_NOTICES.txt')));
assert.ok(fs.existsSync(path.join(folder,'LLAMA_SETTINGS_GUIDE.md')));
assert.equal(fs.existsSync(location),false,'Use a fresh extracted package');
const socket=net.createServer();await new Promise(r=>socket.listen(0,'127.0.0.1',r));const port=socket.address().port;await new Promise(r=>socket.close(r));
const env={...process.env,LOCALAPPDATA:path.join(folder,'test-legacy-context'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:virtualCi?'--disable-gpu --no-sandbox --force-renderer-accessibility':`--remote-debugging-port=${port}`};
for(const key of ['LLAMA_LAUNCHER_DATA_DIR','WEBVIEW2_BROWSER_EXECUTABLE_FOLDER','WEBVIEW2_USER_DATA_FOLDER','LLAMA_MCP_PROFILE','HF_TOKEN'])delete env[key];
fs.mkdirSync(env.LOCALAPPDATA,{recursive:true});
// Hosted Windows does not expose the WebView debugging endpoint reliably.
// Its native accessibility provider verifies the real UI without a CDP port.
// Software rendering/accessibility switches apply only to this isolated test.
const child=spawn(executable,[],{env,windowsHide:true,stdio:['ignore','pipe','pipe']});
let browser,spawnError='',nativeOutput='';
child.on('error',error=>{spawnError=error.message;});
for(const stream of [child.stdout,child.stderr])stream?.on('data',bytes=>{nativeOutput=(nativeOutput+bytes.toString('utf8')).slice(-16384);});
async function until(fn){
  const deadline=Date.now()+60000;let lastError;
  while(Date.now()<deadline){
    if(spawnError||child.exitCode!==null||child.signalCode)throw Error(`Launcher exited: ${spawnError||child.exitCode||child.signalCode}`);
    try{const result=await fn();if(result)return result;}catch(error){lastError=error;}
    await new Promise(r=>setTimeout(r,200));
  }
  throw lastError||Error('Packaged GUI did not initialize');
}
try{
  if(virtualCi){
    const probe=await promisify(execFile)('powershell.exe',['-NoProfile','-STA','-ExecutionPolicy','Bypass','-File',path.join(import.meta.dirname,'native-window-ready.ps1'),'-RootProcessId',String(child.pid),'-RuntimeFolder',path.join(folder,'WebView2')],{env,windowsHide:true,timeout:65000,maxBuffer:1024*1024});
    const report=JSON.parse(probe.stdout.trim());
    assert.equal(report.ready,true);assert.equal(report.startButton,true);assert.equal(report.packagedRuntime,true);
    await until(()=>fs.existsSync(path.join(location,'working-settings.bin')));
    assert.ok(fs.existsSync(path.join(location,'webview')));
    console.log(JSON.stringify({freshPackagedGui:true,packagedFixedWebView:true,dataBesideExecutable:true,privateWorkingSave:true,dependencyNotices:true,nativeReadyStatus:true,method:report.method}));
  }else{
    browser=await until(()=>chromium.connectOverCDP(`http://127.0.0.1:${port}`,{timeout:5000}));
    const page=await until(()=>browser.contexts()[0]?.pages()[0]);
    await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
    assert.equal(new URL(page.url()).origin,'https://tauri.localhost','Keep the Tauri 1 storage origin during migration');
    const actual=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('storage_location'));
    assert.equal(path.resolve(actual),location);assert.ok(fs.existsSync(path.join(location,'webview')));
    assert.equal(await page.locator('#start_server').isVisible(),true);
    assert.equal(await page.locator('#option_grid .option-card').count(),0,'Fresh Launch does not build hidden settings controls');
    await page.getByRole('button',{name:'Settings',exact:true}).click();
    await page.getByRole('button',{name:'Context & cache',exact:true}).click();
    const cache=page.locator('.option-card').filter({has:page.locator('.option-category').filter({hasText:/^--cache-type-v ·/})});
    await cache.locator('.impact-details summary').click();await cache.locator('.impact-details dd').first().waitFor();
    assert.match(await cache.textContent(),/77% combined/);assert.equal(await cache.locator('.impact-details dd').count(),6);
    const blocked=await page.evaluate(async()=>{try{await window.__TAURI_INTERNALS__.invoke('open_llama_documentation',{url:'https://example.com/unrelated'});return false;}catch(e){return String(e).includes('Only official llama.cpp');}});
    assert.equal(blocked,true);
    console.log(JSON.stringify({freshPackagedGui:true,packagedFixedWebView:true,dataBesideExecutable:true,dependencyNotices:true,reviewedSettingsHelp:true,nativeDocumentationValidation:true,method:'CDP'}));
  }
}catch(error){
  const diagnostic={pid:child.pid,exitCode:child.exitCode,signal:child.signalCode,spawnError,error:String(error),stderr:error.stderr,nativeOutput,profileCreated:fs.existsSync(path.join(location,'webview')),virtualCi};
  fs.mkdirSync('artifacts',{recursive:true});fs.writeFileSync('artifacts/native-package-startup.json',JSON.stringify(diagnostic,null,2));
  throw Error('Packaged GUI smoke failed: '+JSON.stringify(diagnostic));
}finally{await browser?.close().catch(()=>{});child.kill();}
