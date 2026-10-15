// Invoked only by the trusted main-branch release job after both platforms pass.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {Readable} from 'node:stream';
import {pipeline} from 'node:stream/promises';
import {execFileSync} from 'node:child_process';

const repository='AlexStapleton/LLamaLauncher',tag='1.1';
const token=process.env.GITHUB_TOKEN,sha=process.env.GITHUB_SHA;
if(!token||process.env.GITHUB_REPOSITORY!==repository||process.env.GITHUB_REF!=='refs/heads/main'||process.env.GITHUB_EVENT_NAME!=='push'||!/^[a-f0-9]{40}$/.test(sha||''))throw Error('Expected a trusted main-branch GitHub release job');
if(JSON.parse(fs.readFileSync('tauri-app/package.json','utf8')).version!=='1.1.0')throw Error('Release version mismatch');
const headers={Authorization:`Bearer ${token}`,Accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28'};
async function api(suffix,method='GET',body){
  const response=await fetch(`https://api.github.com/repos/${repository}/${suffix}`,{method,headers:{...headers,...(body?{'Content-Type':'application/json'}:{})},body:body?JSON.stringify(body):undefined});
  if(!response.ok)throw Error(`GitHub ${method} ${suffix}: ${response.status} ${await response.text()}`);
  return response.status===204?null:response.json();
}
async function hash(file){const digest=createHash('sha256');for await(const chunk of fs.createReadStream(file))digest.update(chunk);return digest.digest('hex');}
const artifacts=await api(`actions/runs/${process.env.GITHUB_RUN_ID}/artifacts?per_page=100`),assets=[];
for(const platform of ['Windows','Linux']){
  const artifact=artifacts.artifacts.find(a=>a.name===`release-assets-${platform}`&&!a.expired);if(!artifact)throw Error(`Missing verified ${platform} artifact`);
  const folder=path.resolve(`release-publish/${platform}`);fs.mkdirSync(folder,{recursive:true});const archive=path.join(folder,'artifact.zip');
  const response=await fetch(`https://api.github.com/repos/${repository}/actions/artifacts/${artifact.id}/zip`,{headers});if(!response.ok||!response.body)throw Error('Cannot download verified workflow artifact');
  await pipeline(Readable.fromWeb(response.body),fs.createWriteStream(archive));
  const name=`LLamaLauncher-${tag}-${platform}-x86_64.zip`,allowed=new Set([name,name+'.sha256']);
  const entries=execFileSync('unzip',['-Z','-1',archive],{encoding:'utf8'}).trim().split(/\r?\n/);if(entries.length!==2||entries.some(e=>!allowed.has(e)))throw Error('Unexpected release artifact entries');
  execFileSync('unzip',['-o',archive,'-d',folder],{stdio:'ignore'});
  const zip=path.join(folder,name),digest=await hash(zip),checksum=fs.readFileSync(zip+'.sha256','utf8').replace(/^\uFEFF/,'').trim();
  if(checksum!==`${digest}  ${name}`)throw Error('Release archive checksum mismatch');
  for(const file of [zip,zip+'.sha256'])assets.push({file,name:path.basename(file),digest:await hash(file),size:fs.statSync(file).size});
}
const sums='release-publish/SHA256SUMS.txt';fs.writeFileSync(sums,assets.filter(a=>a.name.endsWith('.zip')).map(a=>`${a.digest}  ${a.name}`).join('\n')+'\n');assets.push({file:sums,name:'SHA256SUMS.txt',digest:await hash(sums),size:fs.statSync(sums).size});
const lookup=await fetch(`https://api.github.com/repos/${repository}/releases/tags/${tag}`,{headers});
let release;
if(lookup.status===404)release=await api('releases','POST',{tag_name:tag,target_commitish:sha,name:`LLamaLauncher ${tag}`,body:fs.readFileSync('RELEASE_NOTES_1.1.md','utf8')+`\n\nBuilt from commit ${sha}. Both platform jobs passed before publication.`,draft:true,prerelease:true,make_latest:'false'});
else {if(!lookup.ok)throw Error('Cannot inspect release');release=await lookup.json();if(release.target_commitish!==sha)throw Error('Existing release targets another commit');}
for(const asset of assets){
  const existing=release.assets.find(a=>a.name===asset.name);
  if(existing){if(existing.size!==asset.size||existing.digest!==`sha256:${asset.digest}`)throw Error(`Existing release asset differs: ${asset.name}`);continue;}
  if(!release.draft)throw Error('Published release is missing assets; refusing to mutate it');
  const response=await fetch(`https://uploads.github.com/repos/${repository}/releases/${release.id}/assets?${new URLSearchParams({name:asset.name})}`,{method:'POST',headers:{...headers,'Content-Type':asset.name.endsWith('.zip')?'application/zip':'text/plain','Content-Length':String(asset.size)},body:fs.createReadStream(asset.file),duplex:'half'});
  if(!response.ok)throw Error(`Asset upload ${asset.name}: ${response.status}`);const uploaded=await response.json();
  if(uploaded.size!==asset.size||uploaded.digest!==`sha256:${asset.digest}`)throw Error('GitHub asset digest mismatch');
}
release=await api(`releases/${release.id}`);for(const asset of assets){const uploaded=release.assets.find(a=>a.name===asset.name);if(uploaded?.digest!==`sha256:${asset.digest}`)throw Error('Release asset verification failed');}
if(release.draft)release=await api(`releases/${release.id}`,'PATCH',{draft:false,prerelease:true,make_latest:'false'});
console.log(JSON.stringify({release:release.html_url,tag,sha,prerelease:release.prerelease,assets:assets.map(a=>({name:a.name,sha256:a.digest}))}));
