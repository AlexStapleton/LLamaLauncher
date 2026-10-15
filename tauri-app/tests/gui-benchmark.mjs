// Repeated headless browser measurements with a mocked backend, not native GUI/inference benchmarks.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import http from 'node:http';
import { chromium } from 'playwright';
import { parseHelp } from '../src/frontend/option-catalog.js';
const root=path.resolve(import.meta.dirname,'..');
const dist=process.env.LLAMA_BENCH_DIST||path.join(root,'dist');
const help=fs.readFileSync(path.join(root,'src/frontend/server-help.txt'),'utf8');
const expectedSearch = parseHelp(help).filter(option=>option.searchText.includes('reasoning-budget')).length;
const server=http.createServer((request,response)=>{
  const file=path.resolve(dist,'.'+new URL(request.url,'http://localhost').pathname.replace(/\/$/,'/index.html'));
  if(!file.startsWith(path.resolve(dist)+path.sep)){response.writeHead(403).end();return;}
  try{response.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html');response.end(fs.readFileSync(file));}catch{response.writeHead(404).end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const browser=await chromium.launch({headless:true});
const runs=[];
try{
  for(let run=0;run<5;run++){
    const page=await browser.newPage({viewport:{width:1200,height:700}});
    await page.addInitScript(({help})=>{
      window.__TAURI_INVOKE__=async(command)=>{
        if(command==='platform_info')return{windows:true};
        if(command==='profiles_list')return[];
        if(command==='monitoring_get')return {enabled:true,interval_seconds:2};
        if(['monitoring_history','artifacts_list','downloads_list','jobs_list','results_list'].includes(command))return [];
        if(command==='hub_account')return {connected:false};
        if(command==='get_defaults')return{schema_version:2,llama_cpp_path:'',models_path:'',model_file:'',host:'127.0.0.1',port:8080,argument_overrides:[],additional_args:[],router_mode:false,mcp_profile:{name:'fixture',transport:'http',url:'http://localhost:9811/mcp',enabled:false}};
        if(command==='storage_issues')return [];
        if(command==='finish_close'||command==='close_ready')return null;
        if(command==='working_load')return null;
        if(command==='working_save')return null;
        if(command==='storage_location')return 'C:\\fixture\\data';
        if(command==='server_help')return help;
        if(command==='command_preview')return[];
        if(command==='server_status')return'stopped';
        if(command==='list_models')return[];
        if(command==='server_logs')return'';
        if(command==='tauri'||command==='cancel_models')return null;
        throw new Error(`Unexpected benchmark command ${command}`);
      };
    },{help});
    const before=performance.now();
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
    const readyMs=performance.now()-before;
    const tab=performance.now();
    await page.getByRole('button',{name:'Settings',exact:true}).click();
    await page.getByRole('button',{name:'All options',exact:false}).click();
    await page.waitForFunction(()=>document.querySelectorAll('.option-card').length>200);
    const tabMs=performance.now()-tab;
    const search=performance.now();
    await page.locator('#option_search').fill('reasoning-budget');
    await page.waitForFunction(expected=>document.querySelectorAll('.option-card').length===expected,expectedSearch);
    const searchMs=performance.now()-search;
    await page.locator('#option_search').fill('');
    await page.waitForFunction(()=>document.querySelectorAll('.option-card').length>200);
    await page.evaluate(async()=>{
      window.__benchRenders=0;
      window.__benchObserver=new MutationObserver(records=>{window.__benchRenders+=records.filter(record=>record.type==='childList').length;});
      window.__benchObserver.observe(document.getElementById('option_grid'),{childList:true});
      const input=document.getElementById('option_search');input.value='';
      for(const character of 'reasoning-budget'){input.value+=character;input.dispatchEvent(new Event('input'));await new Promise(resolve=>setTimeout(resolve,10));}
    });
    await page.waitForFunction(expected=>document.querySelectorAll('.option-card').length===expected,expectedSearch);
    const burstRenderBatches=await page.evaluate(()=>{window.__benchObserver.disconnect();return window.__benchRenders;});
    const cdp=await page.context().newCDPSession(page);
    await cdp.send('HeapProfiler.collectGarbage');
    const heap=await cdp.send('Runtime.getHeapUsage');
    runs.push({readyMs,tabMs,searchMs,burstRenderBatches,jsHeapBytes:heap.usedSize});
    await page.close();
  }
  const summary={};
  for(const key of Object.keys(runs[0])){const samples=runs.map(run=>run[key]).sort((a,b)=>a-b);summary[key]={median:Number(samples[2].toFixed(2)),max:Number(samples[4].toFixed(2))};}
  const report={date:new Date().toISOString(),kind:'headless Chromium, mocked backend; fresh page per run, warm browser; retained JS heap after forced GC',platform:os.platform(),cpu:os.cpus()[0].model,node:process.version,runs,summary};
  fs.mkdirSync(path.join(root,'artifacts'),{recursive:true});
  fs.writeFileSync(path.join(root,'artifacts',process.env.LLAMA_BENCH_NAME||'gui-benchmark.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify(summary));
}finally{await browser.close();server.close();}
