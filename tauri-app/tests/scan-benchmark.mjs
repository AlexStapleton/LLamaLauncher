import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'..');
const folder=path.join(root,'.test-fixtures/scan-benchmark');
for(let directory=0;directory<20;directory++){
  const location=path.join(folder,String(directory));fs.mkdirSync(location,{recursive:true});
  for(let file=0;file<100;file++)fs.writeFileSync(path.join(location,`${file}.txt`),'fixture');
  fs.writeFileSync(path.join(location,'model-00001-of-00002.gguf'),'fixture');
  fs.writeFileSync(path.join(location,'model-00002-of-00002.gguf'),'fixture');
}
const executable=process.env.LLAMA_LAUNCHER_EXE||path.join(root,'desktop-app',process.platform==='win32'?'llama-server-configurator.exe':'llama-server-configurator');
const runs=[];
for(let sample=0;sample<5;sample++){
  const result=spawnSync(executable,['--benchmark-scan',folder],{encoding:'utf8',windowsHide:true});
  assert.equal(result.status,0,result.stderr);
  const report=JSON.parse(result.stdout);assert.equal(report.models,20);runs.push(report.scanMs);
}
const ordered=[...runs].sort((a,b)=>a-b);
const report={kind:'Synthetic local filesystem, 20 folders/2040 files, 20 models; warm filesystem, no inference',runs,medianMs:ordered[2],maxMs:ordered[4]};
fs.mkdirSync(path.join(root,'artifacts'),{recursive:true});
fs.writeFileSync(path.join(root,'artifacts/scan-benchmark.json'),JSON.stringify(report,null,2));
console.log(JSON.stringify(report));
