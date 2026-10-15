// Deterministic UI fixture; networking, streaming and cancellation are also
// exercised independently by native tests. Never use live credentials here.
export async function installFeatureFixture(page, fixtureModels) {
  const commands=['monitoring_get','monitoring_set','monitoring_history','hardware_snapshot','launch_preflight','model_inspect_config','hub_account','hub_connect','hub_test','hub_disconnect','hub_search','hub_details','hub_open_tokens','hub_read_card','downloads_list','download_start','download_resume','download_remove','artifacts_list','runtime_import','runtime_releases','runtime_install','runtime_install_channel','runtime_channel_plan','working_save','working_load','storage_location','storage_issues','finish_close','close_ready','server_models','router_model','inference_test','jobs_list','job_cancel','benchmark_start','results_list'];
  let preferences={enabled:true,interval_seconds:2},account={connected:false,name:'',client_id:''};
  const jobs=[],downloads=[],artifacts=[],routerActions=[];let sequence=0;let working=null;
  const benchmark={timestamp:1700000000,hardware:{cpu:'Fixture CPU'},results:[{profile:'Coding GPU',startup_seconds:0.5,peak_server_ram:1024**3,runs:[{first_token_seconds:0.1,seconds:2,tokens:64,tokens_per_second:32}]}]};
  await page.exposeFunction('__featureCommand',async(command,args)=>{
    switch(command){
      case 'storage_issues':return [];
      case 'finish_close':case 'close_ready':return null;
      case 'working_load':return working;
      case 'working_save':working=structuredClone(args.config);return null;
      case 'storage_location':return 'C:\\Fixtures\\Launcher\\data';
      case 'runtime_channel_plan':return {label:args.channel==='stable'?'Stable':'Beta',version:'fixture-b2',tag:'fixture-b2',backend:'CPU',asset_id:1};
      case 'monitoring_get':return preferences;
      case 'monitoring_set':preferences=structuredClone(args.preferences);return null;
      case 'monitoring_history':return [];
      case 'hardware_snapshot':return {cpu:'Fixture CPU',logical_cores:8,total_ram:32*1024**3,available_ram:16*1024**3,gpus:[]};
      case 'launch_preflight':return {warnings:[]};
      case 'model_inspect_config':return {path:args.config.model_file,architecture:'llama',name:'Fixture model',parameters:7e9,bytes:4*1024**3,context:8192,quantization:'Q4_K',metadata:{'general.architecture':'llama'},warnings:['Approximate memory estimate'],estimate_low:5*1024**3,estimate_high:7*1024**3,resource_estimate:null};
      case 'hub_open_tokens':return null;
      case 'hub_read_card':return 'Fixture model card';
      case 'hub_account':return account;
      case 'hub_connect':if(args.token!=='hf_fixture_token')throw new Error('Invalid fixture token');account={connected:true,name:'fixture-user',client_id:''};return account;
      case 'hub_disconnect':account={connected:false,name:'',client_id:''};return null;
      case 'hub_test':return account;
      case 'hub_search':return {models:[{id:'fixture/text-model',pipeline_tag:'text-generation',downloads:123}],cursor:''};
      case 'hub_details':return {id:args.repo,sha:'a'.repeat(40),files:[{rfilename:'model-00001-of-00002.gguf',size:1024**3},{rfilename:'model-00002-of-00002.gguf',size:1024**3},{rfilename:'mmproj.gguf',size:10*1024**2}],card:{license:'mit'},gated:false,pipeline:'text-generation'};
      case 'download_start':{
        const id=`download-${++sequence}`;downloads.push({id,repo:args.repo,revision:'a'.repeat(40),root:args.root,files:args.files.map(name=>({name,size:1024**3,sha256:''}))});jobs.push({id,kind:'download',phase:'complete',message:'Verified',completed:2*1024**3,total:2*1024**3,result:{model:fixtureModels[0].path,directory:args.root,model_id:'fixture-model'}});return id;
      }
      case 'downloads_list':return downloads;
      case 'download_remove':downloads.splice(downloads.findIndex(p=>p.id===args.id),1);return null;
      case 'artifacts_list':return artifacts;
      case 'runtime_import':{const a={id:'fixture-runtime',kind:'runtime',name:'CPU runtime',path:args.binary,version:'fixture-b1',managed:false,details:{devices:'CPU'}};artifacts.push(a);return a;}
      case 'runtime_install':case 'runtime_install_channel':{const id='install-'+(++sequence);const a={id:'installed-runtime',kind:'runtime',name:'llama.cpp',path:'C:\\Fixtures\\managed-server.exe',version:'fixture-b2',managed:true,details:{devices:'CPU'}};artifacts.push(a);jobs.push({id,kind:'runtime',phase:'complete',message:'Installed',completed:30*1024**2,total:30*1024**2,result:{artifact:a}});return id;}
      case 'runtime_releases':return {releases:[{tag:'fixture-b2',assets:[{id:1,name:'llama-fixture-win-cpu-x64.zip',size:30*1024**2}]}]};
      case 'server_models':return {data:[{id:'fixture-model',status:{value:'loaded'}}]};
      case 'router_model':routerActions.push(args);return {success:true};
      case 'inference_test':{const id=`inference-${++sequence}`;jobs.push({id,kind:'inference',phase:'complete',message:'',completed:0,total:0,result:{text:'Hello from the fixture server.',first_token_seconds:0.1,seconds:1,tokens:20,tokens_per_second:20}});return id;}
      case 'jobs_list':return jobs;
      case 'job_cancel':return null;
      case 'benchmark_start':{const id=`benchmark-${++sequence}`;jobs.push({id,kind:'benchmark',phase:'complete',message:'',completed:args.profiles.length,total:args.profiles.length,result:benchmark});return id;}
      case 'results_list':return [benchmark];
      default:throw new Error(`Unhandled feature fixture: ${command}`);
    }
  });
  await page.addInitScript(commands=>{window.__featureCommands=commands;},commands);
  return {preferences:()=>preferences,routerActions,downloads,jobs};
}
