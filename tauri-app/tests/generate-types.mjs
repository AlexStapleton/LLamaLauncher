import fs from 'node:fs';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const types = {String:'string',bool:'boolean',u16:'number',u32:'number',u64:'number',i32:'number',f64:'number',Value:'Record<string, unknown>','mcp::Profile':'Profile','crate::mcp::Profile':'Profile','serde_json::Value':'Record<string, unknown>'};
function convert(type) {
  if(types[type]) return types[type];
  if(type.startsWith('Option<')) return `${convert(type.slice(7,-1))} | null`;
  if(type.startsWith('Vec<')) return `${convert(type.slice(4,-1))}[]`;
  if(type==='BTreeMap<String, String>') return 'Record<string, string>';
  if(['ArgumentOverride','Profile','Config','DownloadFile'].includes(type)) return type;
  if(type==='crate::config::Config') return 'Config';
  throw new Error(`Unmapped Rust IPC type ${type}`);
}
let output='// Generated from Rust DTOs by npm run types:generate. Do not edit.\nimport type {JobResult} from "./job-results";\n';
for(const [file,names] of [['config.rs',['Config','ArgumentOverride']],['catalog.rs',['ModelInfo']],['main.rs',['PlatformInfo']],['mcp.rs',['Profile','Report','Heartbeat']],['profiles.rs',['LaunchProfile','ProfileSummary']],['monitor.rs',['MonitoringPreferences','HardwareSnapshot','TelemetrySample']],['gguf.rs',['ModelMetadata']],['jobs.rs',['JobReport']],['downloads.rs',['DownloadFile','DownloadPlan']],['registry.rs',['Artifact']]]) {
  const source=fs.readFileSync(path.join(root,'src',file),'utf8');
  for(const name of names) {
    const body=source.match(new RegExp(`(?:pub )?struct ${name} \\{([^}]+)\\}`))?.[1];
    if(!body)throw new Error(`Missing Rust DTO ${name}`);
    output+=`export interface ${name} {\n`;
    for(const match of body.matchAll(/(?:pub )?(\w+): ([^\n]+?),?\s*(?:\n|$)/g)) {
      output+=`  ${match[1]}: ${name==='JobReport'&&match[1]==='result'?'JobResult':convert(match[2].replace(/,$/, '').trim())};\n`;
    }
    output+='}\n';
  }
}
const target=path.join(root,'src/frontend/native-types.ts');
if(process.argv.includes('--check')) {
  if(!fs.existsSync(target)||fs.readFileSync(target,'utf8').replaceAll('\r\n','\n')!==output) throw new Error('Native types are stale; run npm run types:generate');
} else fs.writeFileSync(target,output);
