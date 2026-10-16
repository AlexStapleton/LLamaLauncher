// Read-only review of external build/runtime inputs. Never downloads or runs an
// installer, changes release pins, or silently substitutes a different channel.
import fs from 'node:fs';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'..');
const inputs=JSON.parse(fs.readFileSync(path.join(root,'runtime-inputs.json'),'utf8'));
const get=async(url,json=true)=>{const r=await fetch(url,{headers:{'User-Agent':'LLamaLauncher-dependency-review (https://github.com/AlexStapleton/LLamaLauncher)'},signal:AbortSignal.timeout(30000)});if(!r.ok)throw Error(`Upstream ${r.status}: ${url}`);return json?r.json():r.text();};
const nodeUrl='https://nodejs.org/dist/index.json';
const webviewUrl='https://developer.microsoft.com/en-us/microsoft-edge/webview2/';
const nsisUrl='https://nsis-dev.github.io/release-data/versions.json';
const webkitUrl='https://webkitgtk.org/';
const [nodes,page,nsis,webkit,audit]=await Promise.all([get(nodeUrl),get(webviewUrl,false),get(nsisUrl),get(webkitUrl,false),get('https://crates.io/api/v1/crates/cargo-audit')]);
const tuple=v=>v.replace(/^v/,'').split('.').map(Number);
const newest=(values)=>values.sort((a,b)=>{const x=tuple(a),y=tuple(b);for(let i=0;i<Math.max(x.length,y.length);i++){const d=(y[i]||0)-(x[i]||0);if(d)return d;}return 0;})[0];
const normalized=page.replaceAll('\\u002F','/').replaceAll('\\/','/');
const fixedVersions=[...normalized.matchAll(/Microsoft\.WebView2\.FixedVersionRuntime\.(\d+\.\d+\.\d+\.\d+)\.x64\.cab/g)].map(m=>m[1]);
const nsisVersions=Object.values(nsis.versions).filter(v=>v.channel==='stable').map(v=>v.version);
const webkitVersions=[...webkit.matchAll(/webkitgtk-(\d+\.\d+\.\d+)\.tar/g)].map(m=>m[1]).filter(v=>tuple(v)[1]%2===0);
if(!fixedVersions.length||!nsisVersions.length||!webkitVersions.length)throw Error('Upstream version format changed; review parsers instead of assuming pins are current');
const report={checkedAt:new Date().toISOString(),node:{pinned:fs.readFileSync(path.join(root,'../.nvmrc'),'utf8').trim(),latest:nodes[0].version.replace(/^v/,''),latestLts:nodes.find(n=>n.lts).version,policy:'Stable Current is used only for build/test tooling; Node is not shipped as an application runtime.',source:nodeUrl},webview2:{pinned:inputs.webview2.version,latestFixedX64:newest(fixedVersions),source:webviewUrl},nsis:{pinned:'3.13',latest:newest(nsisVersions),source:nsisUrl},webkit:{reviewedBuildBaseline:'2.52.6',latestStable:newest(webkitVersions),source:webkitUrl,exception:'Ubuntu 24.04 binary compatibility; this older distro engine is a servicing gap, not proof of upstream currency or security.'},cargoAudit:{pinned:'0.22.2',latest:audit.crate.max_stable_version,source:'https://crates.io/api/v1/crates/cargo-audit'},vcRuntime:{pinned:inputs.vcRuntime.version,source:'https://aka.ms/vc14/vc_redist.x64.exe',review:'Check the signed latest vendor installer FileVersion; the package uses an immutable, hash-pinned official URL. Do not execute that installer to check freshness.'}};
fs.mkdirSync(path.join(root,'artifacts/dependency-audit'),{recursive:true});fs.writeFileSync(path.join(root,'artifacts/dependency-audit/upstream-inputs.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
