import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'..');
const metadataIndex=process.argv.indexOf('--metadata');
const target=process.platform==='win32'?'x86_64-pc-windows-msvc':'x86_64-unknown-linux-gnu';
const source=metadataIndex>=0?fs.readFileSync(process.argv[metadataIndex+1],'utf8'):execFileSync('cargo',[...(process.platform==='win32'?['+stable-x86_64-pc-windows-msvc']:[]),'metadata','--locked','--format-version','1','--filter-platform',target],{cwd:root,maxBuffer:32*1024**2,encoding:'utf8'});
const metadata=JSON.parse(source.replace(/^\uFEFF/,''));
let text=`LLamaLauncher third-party notices\nTarget: ${target}\nGenerated from locked dependency metadata. Includes build dependencies.\nllama.cpp is installed separately from its official distribution, which retains its notices.\nMicrosoft WebView2 and VC redistributables remain subject to their accompanying terms.\n\n`;
const packages=metadata.packages.filter(p=>p.name!=='llama-server-configurator').sort((a,b)=>a.name.localeCompare(b.name)||a.version.localeCompare(b.version));
function append(name,version,license,dir,licenseFile,repository){
  text+=`\n${'='.repeat(72)}\n${name} ${version}\nLicense: ${license||'See upstream terms'}\nSource: ${repository||'crates.io / npm registry'}\n`;
  const files=new Set(fs.readdirSync(dir).filter(n=>/^(licen[sc]e|copying|notice|copyright)([._-]|$)/i.test(n)).map(n=>path.join(dir,n)));
  if(licenseFile)files.add(path.resolve(dir,licenseFile));
  for(const file of files){if(!fs.statSync(file).isFile())continue;const bytes=fs.readFileSync(file);if(bytes.length>256*1024)throw Error(`License text too large: ${name}`);text+=`\n--- ${path.basename(file)} ---\n${bytes.toString('utf8')}\n`;}
}
for(const pkg of packages)append(pkg.name,pkg.version,pkg.license,path.dirname(pkg.manifest_path),pkg.license_file,pkg.repository);
const apiDir=path.join(root,'node_modules/@tauri-apps/api');const api=JSON.parse(fs.readFileSync(path.join(apiDir,'package.json'),'utf8'));append(api.name,api.version,api.license,apiDir,null,typeof api.repository==='string'?api.repository:api.repository?.url);
fs.mkdirSync(path.join(root,'artifacts'),{recursive:true});fs.writeFileSync(path.join(root,'artifacts/THIRD_PARTY_NOTICES.txt'),text);console.log(`Generated notices for ${packages.length+1} dependencies.`);
