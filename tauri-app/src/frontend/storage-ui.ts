import { invoke } from './ipc';
export async function initializeStorage(report:(message:unknown,error?:boolean)=>void) {
  const panel=document.createElement('details'); panel.className='card secondary-details'; panel.id='storage_recovery';
  const summary=document.createElement('summary'); summary.textContent='Saved data & recovery'; panel.append(summary);
  const location=document.createElement('p'); location.className='helper'; location.textContent=`Saved data: ${await invoke('storage_location')}`;
  const status=document.createElement('p'); status.id='settings_save_state'; status.setAttribute('role','status');
  const retry=document.createElement('button'); retry.type='button'; retry.id='settings_save_retry'; retry.textContent='Retry saving'; retry.hidden=true;
  const issues=document.createElement('div'); panel.append(location,status,retry,issues); document.getElementById('setup-panel')!.append(panel);
  for (const issue of await invoke('storage_issues')) {
    panel.open=true; const row=document.createElement('div'); row.className='feature-job'; const message=document.createElement('p'); message.textContent=issue.message; row.append(message);
    for (const [action,label] of [['retry','Retry reading'],['restore','Restore backup'],['reset','Reset this record']] as const) {
      const button=document.createElement('button'); button.type='button'; button.textContent=label; button.disabled=action==='restore'&&!issue.backup;
      button.addEventListener('click',async()=> {
        if(!window.confirm(`${label} for ${issue.name}? The launcher will restart. The original record will be retained when restoring or resetting. Unsaved working changes will be replaced.`))return;
        button.disabled=true;
        try { await invoke('storage_recover',{name:issue.name,action}); } catch(error) { message.textContent=String(error); report(error,true); button.disabled=false; }
      }); row.append(button);
    }
    issues.append(row); report(issue.message,true);
  }
}
