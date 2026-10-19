// Generate version metadata without rewriting the protected Tauri configuration.
import fs from 'node:fs';
const version=JSON.parse(fs.readFileSync('package.json','utf8')).version;
if(!/^\d+\.\d+\.\d+$/.test(version))throw Error('Expected a stable release version');
for(const [file,pattern] of [
  ['Cargo.toml',/^(\[package\]\r?\nname = "[^"]+"\r?\nversion = ")[^"]+("\r?$)/m],
  ['tauri.conf.json',/^(  "version": ")[^"]+(",?\r?$)/m],
]){
  const original=fs.readFileSync(file,'utf8');
  if(!pattern.test(original))throw Error(`Cannot locate version in ${file}`);
  const updated=original.replace(pattern,(_,prefix,suffix)=>prefix+version+suffix);
  if(file.endsWith('.json')){
    const before=JSON.parse(original),after=JSON.parse(updated);
    before.version=after.version;
    if(JSON.stringify(before)!==JSON.stringify(after))throw Error('Non-version configuration changed');
  }
  fs.writeFileSync(file,updated);
}
console.log(`Generated native version metadata for ${version}; regenerate Cargo.lock with Cargo.`);
