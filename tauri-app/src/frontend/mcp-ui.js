import { listen } from '@tauri-apps/api/event';
export function initializeMcp({el,node,invoke,getConfig,syncConfig,report}) {
  const initial={name:'docker_gateway',transport:'http',url:'http://localhost:9811/mcp',bearer_token:'',command:'',args:[],env:{},cwd:'',enabled:false};
  let connected=false,working=false,heartbeating=false,dirty=false,tools=[],server='',revision=0;
  listen('mcp-tools',event=>{if(connected&&!dirty){tools=event.payload;renderTools();}}).catch(()=>{});
  const fields=['mcp_name','mcp_transport','mcp_url','mcp_token','mcp_command','mcp_args','mcp_env','mcp_cwd'];
  function status(message) {
    el('mcp_connection_status').textContent=working?'Checking…':connected?'Connected':'Disconnected';
    el('mcp_connection_status').className=`status-pill ${connected?'running':'stopped'}`;
    el('mcp_nav_status').textContent=connected?'Online':'Offline';
    el('mcp_disconnect').disabled=working||!connected;
    el('mcp_connect').disabled=working;el('mcp_test').disabled=working;
    if(message)el('mcp_connection_message').textContent=message;
  }
  function mode() {
    const config=getConfig();
    el('server_mode').value=config.router_mode?'router':'single';
    el('server_mode').options[0].disabled=!!config.mcp_profile?.enabled;
    document.querySelector('.model-picker').hidden=!!config.router_mode;
    el('router_notice').textContent=config.router_mode?'Router mode starts without a fixed model. It advertises this models directory, or the llama.cpp cache when the folder is empty. Models load on demand.':'Single model loads the selected GGUF. Enable router mode for on-demand models and MCP integration.';
  }
  function hydrate() {
    revision++;
    const config=getConfig();config.mcp_profile={...initial,...config.mcp_profile};
    const profile=config.mcp_profile;
    for(const [id,key] of [['mcp_name','name'],['mcp_transport','transport'],['mcp_url','url'],['mcp_token','bearer_token'],['mcp_command','command'],['mcp_cwd','cwd']])el(id).value=profile[key]||'';
    el('mcp_args').value=JSON.stringify(profile.args||[],null,2);el('mcp_env').value=JSON.stringify(profile.env||{},null,2);el('mcp_enabled').checked=!!profile.enabled;
    el('mcp_http_fields').hidden=profile.transport!=='http';el('mcp_stdio_fields').hidden=profile.transport!=='stdio';mode();
  }
  function readProfile() {
    const args=JSON.parse(el('mcp_args').value||'[]'),env=JSON.parse(el('mcp_env').value||'{}');
    if(!Array.isArray(args)||args.some(arg=>typeof arg!=='string'))throw new Error('MCP Arguments must be a JSON array of strings.');
    if(!env||Array.isArray(env)||typeof env!=='object'||Object.values(env).some(value=>typeof value!=='string'))throw new Error('MCP Environment must be a JSON object with string values.');
    const profile={name:el('mcp_name').value.trim(),transport:el('mcp_transport').value,url:el('mcp_url').value.trim(),bearer_token:el('mcp_token').value.trim(),command:el('mcp_command').value.trim(),args,env,cwd:el('mcp_cwd').value.trim(),enabled:el('mcp_enabled').checked};
    getConfig().mcp_profile=profile;return profile;
  }
  function renderTools() {
    const search=el('mcp_tool_search').value.toLowerCase();
    const shown=tools.filter(tool=>`${tool.name} ${tool.description||''}`.toLowerCase().includes(search));
    el('mcp_tool_count').textContent=`${tools.length} ${connected?'available':'discovered'} tools${server?` · ${server}`:''}`;
    const fragment=document.createDocumentFragment();
    for(const tool of shown){
      const row=node('details','mcp-tool');
      const summary=node('summary');summary.append(node('strong','',tool.name),node('span','',tool.description||'No description supplied.'));
      row.append(summary);
      let built=false;
      row.addEventListener('toggle',()=>{if(row.open&&!built){built=true;row.append(node('pre','',JSON.stringify(tool.inputSchema||{},null,2)));}});
      fragment.append(row);
    }
    if(!shown.length)fragment.append(node('p','empty-state',tools.length?'No matching tools.':'Connect or test the server to discover its tools.'));
    el('mcp_tools').replaceChildren(fragment);
  }
  async function action(kind) {
    if(working)return;
    revision++;
    const wasConnected=connected&&!dirty;
    try {
      const profile=readProfile();working=true;status('Initializing MCP and reading tool definitions…');
      const result=await invoke(kind,{profile});
      tools=result.tools||[];server=result.server||'MCP server';
      connected=kind==='mcp_connect'||wasConnected;dirty=false;
      if(profile.transport==='http'&&result.endpoint){profile.url=result.endpoint;el('mcp_url').value=result.endpoint;}
      if(kind==='mcp_connect'){profile.enabled=true;getConfig().router_mode=true;el('mcp_enabled').checked=true;}
      getConfig().mcp_profile=profile;syncConfig();mode();renderTools();
      status(`${kind==='mcp_connect'?'Connected':'Test passed'}: ${server}, ${tools.length} tools. ${kind==='mcp_connect'?'MCP will be included on the next router start.':'No tools were executed.'}`);
      report(kind==='mcp_connect'?`MCP connected: ${tools.length} tools. Router mode is enabled.`:`MCP test passed: ${tools.length} tools discovered; no tools executed.`);
    }catch(error){connected=false;dirty=true;status(String(error));report(error,true);}
    finally{working=false;status();}
  }
  const connect=()=>action('mcp_connect');
  el('mcp_connect').addEventListener('click',connect);
  el('mcp_test').addEventListener('click',()=>action('mcp_test'));
  el('mcp_disconnect').addEventListener('click',async()=>{
    revision++;
    try{working=true;status();await invoke('mcp_disconnect');connected=false;getConfig().mcp_profile.enabled=false;el('mcp_enabled').checked=false;syncConfig();mode();status('Disconnected. MCP is excluded from the next server start.');}
    catch(error){report(error,true);}finally{working=false;status();renderTools();}
  });
  let searchTimer;
  el('mcp_tool_search').addEventListener('input',()=>{clearTimeout(searchTimer);searchTimer=setTimeout(renderTools,100);});
  for(const id of fields)el(id).addEventListener('input',()=>{
    revision++;dirty=true;connected=false;
    try{readProfile();syncConfig();}catch(error){status(error.message);}
    if(id==='mcp_transport'){el('mcp_http_fields').hidden=el(id).value!=='http';el('mcp_stdio_fields').hidden=el(id).value!=='stdio';}
    status('Connection settings changed. Connect or test to verify them.');
  });
  el('mcp_enabled').addEventListener('change',()=>{
    try{readProfile();if(el('mcp_enabled').checked)getConfig().router_mode=true;mode();syncConfig();report('Router/MCP changes apply on the next server start.');}catch(error){report(error,true);}
  });
  el('server_mode').addEventListener('change',()=>{getConfig().router_mode=el('server_mode').value==='router';mode();syncConfig();report('Server mode applies on the next start.');});
  setInterval(async()=>{
    if(document.hidden)return;
    if(!connected||working||dirty||heartbeating)return;
    const current=revision;
    try{heartbeating=true;const result=await invoke('mcp_status');if(current!==revision)return;connected=!!result.connected;if(!connected)status(result.message||'Connection lost. Reconnect to refresh tools.');}
    catch(error){if(current===revision){connected=false;status(error?.message||error);}}finally{heartbeating=false;status();}
  },15000);
  status();renderTools();
  return{hydrate,connect,snapshot:readProfile,prepareProfile:async()=>{revision++;await invoke('mcp_disconnect');connected=false;dirty=true;tools=[];server='';status('Profile settings loaded. Connect or test to verify MCP.');renderTools();}};
}
