// Opt-in real-endpoint verification. Reads LLAMA_MCP_PROFILE from the environment; never prints credentials or invokes tools.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const executable=process.env.LLAMA_LAUNCHER_EXE||path.join(root,'desktop-app/llama-server-configurator.exe');
const reuseSaved = process.env.LLAMA_MCP_USE_SAVED === '1';
const probeStart=performance.now();
assert.ok(reuseSaved || process.env.LLAMA_MCP_PROFILE,'Supply LLAMA_MCP_PROFILE or opt in with LLAMA_MCP_USE_SAVED=1.');
const probe=spawnSync(executable,[reuseSaved?'--probe-saved-mcp':'--probe-mcp'],{encoding:'utf8',windowsHide:true,timeout:25000});
assert.equal(probe.status,0,probe.stderr||'Native MCP probe failed.');
const report=JSON.parse(probe.stdout.trim());assert.equal(report.connected,true);assert.ok(report.tools>0);
const probeMs=performance.now()-probeStart;
if(!reuseSaved){
  const configured=spawnSync(executable,['--configure-mcp'],{encoding:'utf8',windowsHide:true,timeout:10000});
  assert.equal(configured.status,0,'Protected profile configuration failed.');
}
const profilePath=path.join(process.env.LOCALAPPDATA,'LlamaServerConfigurator/mcp-profile.bin');
const profile=reuseSaved?{name:'saved_gateway'}:JSON.parse(process.env.LLAMA_MCP_PROFILE);
if(!reuseSaved&&profile.bearer_token)assert.ok(!fs.readFileSync(profilePath).includes(Buffer.from(profile.bearer_token)),'Profile leaked the bearer token.');
const env={...process.env};delete env.LLAMA_MCP_PROFILE;delete env.LLAMA_MCP_TOKEN;
const bridge=spawn(executable,['--mcp-http-bridge',profilePath],{stdio:['pipe','pipe','pipe'],windowsHide:true,env});
const pending=new Map();let buffered='';
bridge.stdout.on('data',chunk=>{buffered+=chunk.toString();let newline;while((newline=buffered.indexOf('\n'))>=0){const line=buffered.slice(0,newline);buffered=buffered.slice(newline+1);if(!line.trim())continue;const value=JSON.parse(line);const task=pending.get(value.id);if(task){pending.delete(value.id);clearTimeout(task.timer);value.error?task.reject(new Error(`Bridge RPC failed (${value.error.code}).`)):task.resolve(value.result);}}});
bridge.on('exit',()=>{for(const task of pending.values()){clearTimeout(task.timer);task.reject(new Error('Bridge exited before replying.'));}});
function rpc(id,method,params){return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(new Error('Bridge request timed out.'));},20000);pending.set(id,{resolve,reject,timer});bridge.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n');});}
let router;
try {
  const initialized=await rpc(100,'initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'llama-router-integration-check',version:'1'}});
  assert.ok(initialized.protocolVersion);
  bridge.stdin.write(JSON.stringify({jsonrpc:'2.0',method:'notifications/initialized'})+'\n');
  const tools=await rpc(101,'tools/list',{});assert.ok(tools.tools.length>0);
  if(process.env.LLAMA_SERVER_EXE){
    const folder=path.join(root,'.test-fixtures/router-empty');fs.mkdirSync(folder,{recursive:true});
    const config={mcpServers:{[profile.name]:{command:executable,args:['--mcp-http-bridge',profilePath],timeout_ms:30000}}};
    router=spawn(process.env.LLAMA_SERVER_EXE,['--host','127.0.0.1','--port','19811','--models-dir',folder,'--mcp-servers-json',JSON.stringify(config)],{windowsHide:true,stdio:['ignore','pipe','pipe'],env});
    let ready=false;const deadline=Date.now()+20000;
    while(Date.now()<deadline){if(router.exitCode!==null)throw new Error('Router exited during startup.');try{const response=await fetch('http://127.0.0.1:19811/health',{signal:AbortSignal.timeout(1000)});if(response.ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,200));}
    assert.equal(ready,true,'Router did not become healthy.');
    const response=await fetch('http://127.0.0.1:19811/models');assert.equal(response.ok,true);const models=await response.json();assert.ok(Array.isArray(models.data)||Array.isArray(models.models));
  }
  const verification={nativeMcp:true,server:report.server,tools:report.tools,bridgeTools:tools.tools.length,bridgeProtocol:initialized.protocolVersion,routerHealthy:!!router,protectedProfile:true,toolsExecuted:0,probeMs};
  fs.mkdirSync(path.join(root,'artifacts'),{recursive:true});
  fs.writeFileSync(path.join(root,'artifacts/native-integration.json'),JSON.stringify(verification,null,2));
  console.log(JSON.stringify(verification));
}finally{router?.kill();bridge.stdin.end();setTimeout(()=>{bridge.kill();},2000).unref();}
