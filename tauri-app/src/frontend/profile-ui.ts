import { invoke } from './ipc';
import type { Config, ProfileSummary } from './native-types';

function stable(value: unknown): unknown {
  if(Array.isArray(value))return value.map(stable);
  if(value && typeof value==='object')return Object.fromEntries(Object.entries(value).sort(([a],[b])=>a.localeCompare(b)).map(([key,item])=>[key,stable(item)]));
  return value;
}
export function profileFingerprint(config: Config): string { return JSON.stringify(stable(config)); }
interface Context {
  getConfig: ()=>Config;
  snapshot: ()=>Config;
  apply: (config:Config)=>Promise<void>;
  launch: ()=>Promise<void>;
  activeServer: ()=>boolean;
  open: (tab:string)=>void;
  report: (message:string,error?:boolean)=>void;
}
export function initializeProfiles(context:Context) {
  const element=<T extends HTMLElement>(id:string)=>document.getElementById(id) as T;
  const select=(id:string)=>element<HTMLSelectElement>(id);
  const name=element<HTMLInputElement>('profile_name');
  let library:ProfileSummary[]=[],loaded:ProfileSummary|undefined,baseline='',working=false;
  const buttons=['profile_create','profile_update','profile_rename','profile_duplicate','profile_delete','profile_load','profile_launch','profile_refresh','profile_quick_launch'];
  function message(text:string,error=false) { element('profile_message').textContent=text;context.report(text,error); }
  function changed() {
    const dirty=!!baseline&&profileFingerprint(context.getConfig())!==baseline;
    element('profile_active').textContent=loaded?`${loaded.name}${dirty?' · unsaved changes':''}`:`Working settings${dirty?' · unsaved changes':''}`;
  }
  function controls() {
    const selected=!!select('profile_list').value;
    for(const id of buttons)element<HTMLButtonElement>(id).disabled=working||(id!=='profile_create'&&id!=='profile_refresh'&&id!=='profile_quick_launch'&&!selected);
    element<HTMLButtonElement>('profile_quick_launch').disabled=working||!select('profile_quick').value||context.activeServer();
    element<HTMLButtonElement>('profile_launch').disabled=working||!selected||context.activeServer();
  }
  function details() {
    const profile=library.find(item=>item.id===select('profile_list').value);
    const target=element('profile_details');target.replaceChildren();
    if(profile)for(const [key,value] of [['Mode',profile.router?'Router':'Single model'],['Model',profile.model||(profile.router?'llama.cpp model cache':'Model source in saved arguments / not selected')],['Runtime',profile.runtime||'Not selected'],['Settings',`${profile.overrides} overrides · full API, extra arguments and MCP settings saved`]]) {
      const term=document.createElement('dt'),definition=document.createElement('dd');term.textContent=key;definition.textContent=value;target.append(term,definition);
    }
    controls();
  }
  async function refresh(selected=select('profile_list').value) {
    library=(await invoke('profiles_list')).sort((a,b)=>a.name.localeCompare(b.name));
    for(const id of ['profile_list','profile_quick']) {
      const control=select(id),previous=id==='profile_list'?selected:loaded?.id||control.value;
      control.replaceChildren();
      const empty=document.createElement('option');empty.value='';empty.textContent=id==='profile_quick'?'Working settings':library.length?'Select a profile…':'No profiles saved yet';control.append(empty);
      for(const profile of library){const option=document.createElement('option');option.value=profile.id;option.textContent=profile.name;control.append(option);}
      control.value=library.some(item=>item.id===previous)?previous:'';
    }
    details();changed();
  }
  async function confirm(title:string,text:string):Promise<boolean> {
    const dialog=element<HTMLDialogElement>('profile_confirm');
    element('profile_confirm_title').textContent=title;element('profile_confirm_message').textContent=text;
    return await new Promise(resolve=>{
      const accept=element<HTMLButtonElement>('profile_confirm_accept'),cancel=element<HTMLButtonElement>('profile_confirm_cancel');
      const done=(value:boolean)=>{accept.removeEventListener('click',yes);cancel.removeEventListener('click',no);dialog.removeEventListener('cancel',cancelled);dialog.close();resolve(value);};
      const yes=()=>done(true),no=()=>done(false),cancelled=(event:Event)=>{event.preventDefault();done(false);};
      accept.addEventListener('click',yes);cancel.addEventListener('click',no);dialog.addEventListener('cancel',cancelled);dialog.showModal();cancel.focus();
    });
  }
  async function load(id:string,launch=false) {
    if(!id)throw new Error('Select a saved profile first.');
    if(launch&&context.activeServer())throw new Error('Stop the running server before launching another profile.');
    if(baseline&&profileFingerprint(context.getConfig())!==baseline && !await confirm('Replace unsaved settings?','Loading this saved profile replaces your current unsaved settings. Cancel to save them as a profile first.'))return;
    const entry=await invoke('profiles_load',{id});
    await context.apply(entry.config);
    loaded=library.find(item=>item.id===entry.id);baseline=profileFingerprint(entry.config);
    select('profile_list').value=id;select('profile_quick').value=id;name.value=entry.name;
    changed();details();context.open('setup');
    message(`Loaded “${entry.name}”.${context.activeServer()?' Changes apply to the next launch; the running server keeps its settings.':''}`);
    if(launch)await context.launch();
  }
  async function operation(action:()=>Promise<void>) {
    if(working)return;working=true;controls();
    try{await action();}catch(error){message(String(error),true);}finally{working=false;controls();}
  }
  const click=(id:string,action:()=>Promise<void>)=>element(id).addEventListener('click',()=>void operation(action));
  click('profile_create',async()=>{
    const config=structuredClone(context.snapshot());const entry=await invoke('profiles_save',{id:null,name:name.value,config});
    await refresh(entry.id);loaded=library.find(item=>item.id===entry.id);baseline=profileFingerprint(entry.config);select('profile_quick').value=entry.id;changed();message(`Saved “${entry.name}”.`);
  });
  click('profile_update',async()=>{
    const id=select('profile_list').value,profile=library.find(item=>item.id===id);if(!profile)return;
    if(!await confirm('Update saved profile?',`Replace the saved model, runtime and settings in “${profile.name}” with your current settings?`))return;
    const entry=await invoke('profiles_save',{id,name:profile.name,config:structuredClone(context.snapshot())});
    await refresh(id);loaded=library.find(item=>item.id===id);baseline=profileFingerprint(entry.config);select('profile_quick').value=id;changed();message(`Updated “${entry.name}”.`);
  });
  for(const [id,rename] of [['profile_rename',true],['profile_duplicate',false]] as const)click(id,async()=>{
    const original=select('profile_list').value;
    const entry=await invoke('profiles_copy',{id:original,name:name.value,rename});
    if(rename&&loaded?.id===original)loaded={...loaded,name:entry.name};
    await refresh(entry.id);message(`${rename?'Renamed':'Duplicated'} profile as “${entry.name}”.`);
  });
  click('profile_delete',async()=>{
    const id=select('profile_list').value,profile=library.find(item=>item.id===id);if(!profile)return;
    if(!await confirm('Delete saved profile?',`Delete “${profile.name}”? Its saved settings will be removed. Model and executable files stay where they are.`))return;
    await invoke('profiles_delete',{id});if(loaded?.id===id){loaded=undefined;baseline=profileFingerprint(context.getConfig());}
    await refresh('');message(`Deleted “${profile.name}”. Current working settings were kept.`);
  });
  click('profile_load',()=>load(select('profile_list').value));
  click('profile_launch',()=>load(select('profile_list').value,true));
  click('profile_quick_launch',()=>load(select('profile_quick').value,true));
  click('profile_refresh',()=>refresh());
  element('profile_manage').addEventListener('click',()=>context.open('profiles'));
  select('profile_list').addEventListener('change',()=>{const profile=library.find(item=>item.id===select('profile_list').value);name.value=profile?.name||'';details();});
  select('profile_quick').addEventListener('change',()=>void operation(async()=>{const id=select('profile_quick').value;if(id)await load(id);else{loaded=undefined;changed();}select('profile_quick').value=loaded?.id||'';}));
  return { changed, controls, initialize:async()=>{baseline=profileFingerprint(context.getConfig());await refresh();} };
}
