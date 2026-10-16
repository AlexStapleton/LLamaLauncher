// Read-only review of external build/runtime inputs. Never runs an installer,
// changes release pins, or silently substitutes a different channel.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=path.resolve(import.meta.dirname,'..');
const inputs=JSON.parse(fs.readFileSync(path.join(root,'runtime-inputs.json'),'utf8'));
const get=async(url,json=true)=>{const r=await fetch(url,{headers:{'User-Agent':'LLamaLauncher-dependency-review (https://github.com/AlexStapleton/LLamaLauncher)'},signal:AbortSignal.timeout(30000)});if(!r.ok)throw Error(`Upstream ${r.status}: ${url}`);return json?r.json():r.text();};
const nodeUrl='https://nodejs.org/dist/index.json';
const webviewUrl='https://developer.microsoft.com/en-us/microsoft-edge/webview2/';
const nsisUrl='https://nsis-dev.github.io/release-data/versions.json';
const webkitUrl='https://webkitgtk.org/';
const vcUrl='https://aka.ms/vc14/vc_redist.x64.exe';
const vcResponse=await fetch(vcUrl,{signal:AbortSignal.timeout(30000)});
if(!vcResponse.ok||!vcResponse.body)throw Error('Cannot inspect the latest official VC runtime');
const vcChunks=[];let vcBytes=0;
for await(const chunk of vcResponse.body){vcBytes+=chunk.length;if(vcBytes>64*1024**2)throw Error('VC runtime exceeds review size limit');vcChunks.push(chunk);}
const vc=Buffer.concat(vcChunks),vcSha=createHash('sha256').update(vc).digest('hex');
// Read VS_FIXEDFILEINFO inside the PE resource section. This observes vendor
// metadata without executing the installer; Windows signature validation is
// still required before accepting a new pin.
if(vc.toString('ascii',0,2)!=='MZ')throw Error('VC download is not a PE file');
const pe=vc.readUInt32LE(60);
if(pe+24>vc.length||vc.readUInt32LE(pe)!==0x4550)throw Error('Invalid VC PE header');
const sectionTable=pe+24+vc.readUInt16LE(pe+20);let resource;
for(let i=0;i<vc.readUInt16LE(pe+6);i++){
  const offset=sectionTable+i*40;if(offset+40>vc.length)throw Error('Invalid VC section table');
  if(vc.toString('ascii',offset,offset+8).replaceAll('\0','')!=='.rsrc')continue;
  const size=vc.readUInt32LE(offset+16),start=vc.readUInt32LE(offset+20);
  if(start+size>vc.length)throw Error('Invalid VC resource bounds');resource=vc.subarray(start,start+size);
}
const fixed=resource?.indexOf(Buffer.from([0xbd,0x04,0xef,0xfe]));
if(fixed===undefined||fixed<0||fixed+16>resource.length)throw Error('VC version metadata missing');
const ms=resource.readUInt32LE(fixed+8),ls=resource.readUInt32LE(fixed+12);
const vcVersion=[ms>>>16,ms&65535,ls>>>16,ls&65535].join('.');
const [nodes,page,nsis,webkit,audit]=await Promise.all([get(nodeUrl),get(webviewUrl,false),get(nsisUrl),get(webkitUrl,false),get('https://crates.io/api/v1/crates/cargo-audit')]);
const tuple=v=>v.replace(/^v/,'').split('.').map(Number);
const newest=(values)=>values.sort((a,b)=>{const x=tuple(a),y=tuple(b);for(let i=0;i<Math.max(x.length,y.length);i++){const d=(y[i]||0)-(x[i]||0);if(d)return d;}return 0;})[0];
const normalized=page.replaceAll('\\u002F','/').replaceAll('\\/','/');
const fixedVersions=[...normalized.matchAll(/Microsoft\.WebView2\.FixedVersionRuntime\.(\d+\.\d+\.\d+\.\d+)\.x64\.cab/g)].map(m=>m[1]);
const nsisVersions=Object.values(nsis.versions).filter(v=>v.channel==='stable').map(v=>v.version);
const webkitVersions=[...webkit.matchAll(/webkitgtk-(\d+\.\d+\.\d+)\.tar/g)].map(m=>m[1]).filter(v=>tuple(v)[1]%2===0);
if(!fixedVersions.length||!nsisVersions.length||!webkitVersions.length)throw Error('Upstream version format changed; review parsers instead of assuming pins are current');
const report={checkedAt:new Date().toISOString(),node:{pinned:fs.readFileSync(path.join(root,'../.nvmrc'),'utf8').trim(),latest:nodes[0].version.replace(/^v/,''),latestLts:nodes.find(n=>n.lts).version,policy:'Stable Current is used only for build/test tooling; Node is not shipped as an application runtime.',source:nodeUrl},webview2:{pinned:inputs.webview2.version,latestFixedX64:newest(fixedVersions),source:webviewUrl},nsis:{pinned:'3.13',latest:newest(nsisVersions),source:nsisUrl},webkit:{reviewedBuildBaseline:'2.52.6',latestStable:newest(webkitVersions),source:webkitUrl,exception:'Ubuntu 24.04 binary compatibility; this older distro engine is a servicing gap, not proof of upstream currency or security.'},cargoAudit:{pinned:'0.22.2',latest:audit.crate.max_stable_version,source:'https://crates.io/api/v1/crates/cargo-audit'},vcRuntime:{pinned:inputs.vcRuntime.version,source:'https://aka.ms/vc14/vc_redist.x64.exe',review:'Check the signed latest vendor installer FileVersion; the package uses an immutable, hash-pinned official URL. Do not execute that installer to check freshness.'}};
report.vcRuntime={pinned:inputs.vcRuntime.version,latestObserved:vcVersion,latestSha256:vcSha,matchesPinned:vcSha===inputs.vcRuntime.sha256,source:vcUrl,resolvedSource:vcResponse.url,review:'Version/hash observed without executing the installer. Windows publisher signature and architecture verification remain mandatory before adopting a changed pin.'};
report.webkit={reviewedBuildBaseline:'2.54.0',latestStable:newest(webkitVersions),source:webkitUrl,exception:'Debian 13 security/updates package line; 2.54.1 is newer upstream. Record exact vendor patches and review native advisory coverage instead of claiming all upstream fixes are present.'};
fs.mkdirSync(path.join(root,'artifacts/dependency-audit'),{recursive:true});fs.writeFileSync(path.join(root,'artifacts/dependency-audit/upstream-inputs.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
