import assert from 'node:assert/strict';
export async function reviewUiChecks(page,openSection,fixture) {
  await openSection('Downloads & tasks');
  const job={id:'focus-fixture',kind:'download',phase:'running',message:'Starting',completed:0,total:100,result:{}};
  fixture.jobs.push(job);await page.evaluate(report=>window.__emit('launcher-job',report),job);
  await page.locator('[data-job-id="focus-fixture"]').getByRole('button',{name:'Pause',exact:true}).focus();
  await page.evaluate(()=>window.__focusedProgressButton=document.activeElement);
  job.completed=50;job.message='Half complete';await page.evaluate(report=>window.__emit('launcher-job',report),job);
  assert.equal(await page.evaluate(()=>window.__focusedProgressButton===document.activeElement&&document.activeElement.isConnected),true);fixture.jobs.pop();
  await page.getByRole('button',{name:/^Connections/}).click();
  await page.evaluate(()=>{window.__reviewOriginal=window.__TAURI_INVOKE__;window.__TAURI_INVOKE__=async(command,args)=>command==='mcp_connect'?{server:'Large fixture',endpoint:args.profile.url,tools:Array.from({length:10000},(_,i)=>({name:`tool_${String(i).padStart(5,'0')}`,inputSchema:{type:'object'}}))}:window.__reviewOriginal(command,args);});
  await page.locator('#mcp_connect').click();await page.waitForFunction(()=>document.querySelector('#mcp_tool_count').textContent.includes('10000'));
  assert.equal(await page.locator('.mcp-tool').count(),100);await page.getByRole('button',{name:/Show more/}).click();assert.equal(await page.locator('.mcp-tool').count(),200);
  await page.locator('#mcp_tool_search').fill('tool_09999');await page.waitForFunction(()=>document.querySelectorAll('.mcp-tool').length===1);
  await page.evaluate(()=>{window.__TAURI_INVOKE__=async(command,args)=>command==='mcp_connect'?new Promise(resolve=>window.__finishDelayedMcp=resolve):window.__reviewOriginal(command,args);});
  await page.locator('#mcp_connect').click();await page.waitForFunction(()=>!!window.__finishDelayedMcp);assert.equal(await page.locator('#mcp_url').isDisabled(),true);
  await page.evaluate(async()=>{const defaults=await window.__reviewOriginal('get_defaults');window.__savedConfig={...defaults,mcp_profile:{...defaults.mcp_profile,url:'http://new-fixture.invalid/mcp'}};document.getElementById('load_config').click();});
  await page.waitForFunction(()=>document.getElementById('mcp_url').value==='http://new-fixture.invalid/mcp');
  await page.evaluate(()=>window.__finishDelayedMcp({server:'Stale fixture',endpoint:'http://old-fixture.invalid/mcp',tools:[{name:'stale_tool',inputSchema:{type:'object'}}]}));
  await page.waitForFunction(()=>!document.getElementById('mcp_connect').disabled);assert.equal(await page.locator('#mcp_url').inputValue(),'http://new-fixture.invalid/mcp');assert.equal(await page.locator('.mcp-tool').count(),0);
  await page.evaluate(()=>{window.__failSave=true;window.__TAURI_INVOKE__=async(command,args)=>{
    if(command==='working_save'&&window.__failSave)throw Error('Denied fixture write');
    if(command==='working_save'&&window.__delaySave)return new Promise(resolve=>{window.__savePending??=[];window.__savePending.push(async()=>{await window.__reviewOriginal(command,args);resolve(null);});});
    if(command==='finish_close')window.__closeFinished=true;return window.__reviewOriginal(command,args);
  };});
  await openSection('Setup');await page.locator('#host').fill('127.0.0.2');await page.waitForFunction(()=>document.getElementById('settings_save_state').textContent.includes('not saved'));
  await page.evaluate(()=>window.__failSave=false);await openSection('Setup');await page.locator('#storage_recovery > summary').click();await page.locator('#settings_save_retry').click();await page.waitForFunction(()=>document.getElementById('settings_save_state').textContent==='Working settings saved.');
  await page.evaluate(()=>{window.__delaySave=true;window.__closeFinished=false;});await openSection('Setup');await page.locator('#host').fill('127.0.0.3');await page.waitForFunction(()=>window.__savePending?.length===1);
  await page.locator('#host').fill('127.0.0.4');await page.evaluate(()=>{window.__emit('launcher-close-request',null);});assert.equal(await page.evaluate(()=>window.__closeFinished),false);
  await page.evaluate(()=>window.__savePending.shift()());await page.waitForFunction(()=>window.__savePending.length===1);assert.equal(await page.evaluate(()=>window.__closeFinished),false);
  await page.evaluate(()=>{window.__delaySave=false;window.__savePending.shift()();});await page.waitForFunction(()=>window.__closeFinished);assert.equal(await page.evaluate(()=>window.__reviewOriginal('working_load').then(c=>c.host)),'127.0.0.4');
}
