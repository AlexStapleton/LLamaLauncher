// Online inventory of every locked package, including optional/foreign-target
// packages. Registry freshness is separate from compatibility and security.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {setTimeout as delay} from 'node:timers/promises';

const root=path.resolve(import.meta.dirname,'..');
const outputIndex=process.argv.indexOf('--output');
const output=path.resolve(root,outputIndex<0?'artifacts/dependency-audit/dependencies.json':process.argv[outputIndex+1]);
const run=(args)=>execFileSync('cargo',[...(process.platform==='win32'?['+stable-x86_64-pc-windows-msvc']:[]),...args],{cwd:root,encoding:'utf8',windowsHide:true,maxBuffer:64*1024**2});
const metadata=JSON.parse(run(['metadata','--locked','--format-version','1']));
const npmLock=JSON.parse(fs.readFileSync(path.join(root,'package-lock.json'),'utf8'));
const version=(v)=>{const core=v.split('+')[0];return /^\d+\.\d+\.\d+$/.test(core)?core.split('.').map(Number):null;};
const compare=(a,b)=>{const x=version(a),y=version(b);if(!x||!y)throw Error(`Expected stable versions: ${a}, ${b}`);for(let i=0;i<3;i++)if(x[i]!==y[i])return Math.sign(x[i]-y[i]);return 0;};
async function get(url,text=false){
  for(let attempt=0;attempt<4;attempt++){
    const response=await fetch(url,{headers:{'User-Agent':'LLamaLauncher-dependency-review (https://github.com/AlexStapleton/LLamaLauncher)'},signal:AbortSignal.timeout(30000)});
    if(response.ok)return text?response.text():response.json();
    if(![429,502,503,504].includes(response.status)||attempt===3)throw Error(`Registry ${response.status}: ${url}`);
    await delay(Math.min(15000,Math.max(1000,Number(response.headers.get('retry-after')||attempt+1)*1000)));
  }
}
const rustPackages=metadata.packages.filter(p=>p.source?.startsWith('registry+')||p.name==='glib');
const cargoNames=[...new Set(rustPackages.map(p=>p.name))].sort();
const npmPackages=Object.entries(npmLock.packages).filter(([key,p])=>key&&p.version).map(([key,p])=>({key,name:key.split('node_modules/').at(-1),...p}));
const npmNames=[...new Set(npmPackages.map(p=>p.name))].sort();
const latestRust=new Map(),latestNpm=new Map();
const queue=[...cargoNames.map(name=>({ecosystem:'cargo',name})),...npmNames.map(name=>({ecosystem:'npm',name}))];
let done=0;
await Promise.all(Array.from({length:4},async()=>{
  while(queue.length){
    const {ecosystem,name}=queue.shift();
    if(ecosystem==='cargo'){
      const prefix=name.length===1?'1':name.length===2?'2':name.length===3?`3/${name[0]}`:`${name.slice(0,2)}/${name.slice(2,4)}`;
      const url=`https://index.crates.io/${prefix}/${name}`;
      const releases=(await get(url,true)).trim().split('\n').map(line=>JSON.parse(line)).filter(r=>!r.yanked&&version(r.vers)).sort((a,b)=>compare(b.vers,a.vers));
      if(!releases.length)throw Error(`No stable registry release: ${name}`);
      const latest=releases[0];
      latestRust.set(name,{latest:latest.vers,latestRustVersion:latest.rust_version||null,source:url,releases});
    }else{
      const url=`https://registry.npmjs.org/${encodeURIComponent(name)}/latest`;
      const latest=await get(url);
      if(!version(latest.version))throw Error(`npm latest is not stable: ${name}`);
      latestNpm.set(name,{latest:latest.version,latestNodeRange:latest.engines?.node||null,source:url});
    }
    done++;if(done%50===0)console.log(`Checked ${done} registry packages`);
    await delay(200);
  }
}));
const byId=new Map(metadata.packages.map(p=>[p.id,p]));
const parents=new Map();
for(const node of metadata.resolve.nodes){
  const parent=byId.get(node.id);
  for(const edge of node.deps){
    const requirements=parent.dependencies.filter(d=>(d.rename||d.name).replaceAll('-','_')===edge.name).map(d=>d.req);
    const rows=parents.get(edge.pkg)||[];
    rows.push({name:parent.name,version:parent.version,requirements:[...new Set(requirements)]});parents.set(edge.pkg,rows);
  }
}
const cargo=rustPackages.map(p=>{
  const {releases,...latest}=latestRust.get(p.name);
  const locked=releases.find(r=>r.vers===p.version);
  return {name:p.name,locked:p.version,...latest,outdated:compare(p.version,latest.latest)<0,localPatch:!p.source,lockedYanked:!locked&&Boolean(p.source),parents:parents.get(p.id)||[]};
}).sort((a,b)=>a.name.localeCompare(b.name)||compare(a.locked,b.locked));
const npm=npmPackages.map(p=>{
  const latest=latestNpm.get(p.name);
  const rows=Object.entries(npmLock.packages).flatMap(([key,other])=>{
    const requirement=other.dependencies?.[p.name]||other.optionalDependencies?.[p.name];
    return requirement?[{name:key?key.split('node_modules/').at(-1):npmLock.name,version:other.version,requirement}]:[];
  });
  return {name:p.name,locked:p.version,...latest,outdated:compare(p.version,latest.latest)<0,optional:Boolean(p.optional),parents:rows};
}).sort((a,b)=>a.name.localeCompare(b.name)||compare(a.locked,b.locked));
const report={checkedAt:new Date().toISOString(),policy:'Latest stable registry releases. An older version needs a compatibility/security review; this inventory alone does not approve upgrades or prove security.',scope:'All locked Rust target platforms and npm optional platform packages; does not include absent optional peer dependencies, native system packages or external downloaded runtimes.',tools:{node:process.version,cargo:run(['--version']).trim()},counts:{cargo:cargo.length,npm:npm.length,cargoOutdated:cargo.filter(p=>p.outdated).length,npmOutdated:npm.filter(p=>p.outdated).length},cargo,npm};
fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify(report,null,2)+'\n');
const cell=value=>String(value).replaceAll('|','\\|').replaceAll('\n',' ');
const table=(items)=>['| Package | Locked | Latest stable | Parents and declared constraints |','| --- | --- | --- | --- |',...items.filter(p=>p.outdated).map(p=>`| ${cell(p.name)} | ${cell(p.locked)}${p.localPatch?' (source backport)':''} | ${cell(p.latest)} | ${p.parents.map(q=>cell(`${q.name} ${q.version}: ${(q.requirements||[q.requirement]).join(', ')}`)).join('; ')} |`)].join('\n');
const markdown=`# Locked dependency inventory\n\nChecked ${report.checkedAt}. Generated by \`node tests/dependency-inventory.mjs\`; the adjacent JSON contains every inspected package, including packages already current.\n\n${report.counts.cargo} Rust instances and ${report.counts.npm} npm packages were checked. ${report.counts.cargoOutdated} Rust instances and ${report.counts.npmOutdated} npm packages are behind the registry's latest stable release. Different locked versions count separately. Foreign-target packages are included; this is not a list of everything shipped in either binary. Freshness is separate from vulnerabilities.\n\nThe constraints below identify which parents require an older release line. They are evidence for a compatibility review, not blanket approval to retain a vulnerable package. See [dependency review](DEPENDENCY_REVIEW.md) for decisions, native/runtime inputs and validation.\n\n## Rust versions behind upstream\n\n${table(cargo)}\n\n## npm versions behind upstream\n\n${table(npm)}\n`;
fs.writeFileSync(output.replace(/\.json$/,'.md'),markdown);
console.log(JSON.stringify({output,counts:report.counts,outdated:[...cargo,...npm].filter(p=>p.outdated).map(p=>({name:p.name,locked:p.locked,latest:p.latest}))}));
