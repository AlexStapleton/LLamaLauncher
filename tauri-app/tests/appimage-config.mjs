import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
// linuxdeploy excludes common desktop and support libraries. Include these
// explicitly; clean Debian also lacks the crypto/error helpers Ubuntu supplies.
// Keep the bundler's platform/graphics-driver exclusions intact.
const names=['libfribidi.so.0','libharfbuzz.so.0','libharfbuzz-icu.so.0','libgraphite2.so.3','libthai.so.0','libdatrie.so.1','libgpg-error.so.0','libcom_err.so.2'];
const cache=execFileSync('ldconfig',['-p'],{encoding:'utf8'}),files={};
fs.mkdirSync('artifacts/linux-library-licenses',{recursive:true});
let report='\nExplicit bundled portability libraries (SHA256):\n';
for(const name of names){
  const row=cache.split('\n').find(line=>line.trim().startsWith(name+' ')&&line.includes('x86-64'));
  if(!row){if(name==='libharfbuzz-icu.so.0')continue;throw Error(`Missing build dependency ${name}`);}
  const source=fs.realpathSync(row.split('=>')[1].trim());
  if(!source.startsWith('/usr/lib/')&&!source.startsWith('/lib/'))throw Error('Unexpected system library path');
  files[`usr/lib/${name}`]=source;
  report+=`${createHash('sha256').update(fs.readFileSync(source)).digest('hex')}  ${name}\n`;
  let owner;
  for(const candidate of new Set([source,source.replace(/^\/usr\/lib\//,'/lib/')])){
    try{owner=execFileSync('dpkg-query',['-S',candidate],{encoding:'utf8',stdio:['ignore','pipe','ignore']}).split(': /')[0].trim();break;}catch{}
  }
  if(!owner||!/^[a-z0-9.+:-]+$/.test(owner))throw Error(`Cannot determine license owner for ${name}`);
  const pkg=owner.split(':')[0],copyright=`/usr/share/doc/${pkg}/copyright`;
  fs.copyFileSync(copyright,path.join('artifacts/linux-library-licenses',pkg+'.copyright'));
}
fs.appendFileSync('artifacts/LINUX_PACKAGE_VERSIONS.txt',report);
fs.writeFileSync('artifacts/portable-config.json',JSON.stringify({bundle:{active:true,targets:['appimage'],icon:['icons/icon.png'],linux:{appimage:{files}},resources:['../LICENSE','../PORTABLE_RELEASES.md','../LLAMA_SETTINGS_GUIDE.md','artifacts/THIRD_PARTY_NOTICES.txt','artifacts/LINUX_PACKAGE_VERSIONS.txt','artifacts/linux-library-licenses/']}},null,2)+'\n');
console.log(report);
