import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const root=path.resolve(import.meta.dirname,'../../vendor/glib-0.15.12');
const manifest=JSON.parse(fs.readFileSync(path.join(root,'UPSTREAM_SHA256.json'),'utf8'));
for(const [name,digest] of Object.entries(manifest.files)){
  const actual=crypto.createHash('sha256').update(fs.readFileSync(path.join(root,name))).digest('hex');
  if(actual!==digest)throw Error(`Vendored glib changed outside the verified backport: ${name}`);
}
const source=fs.readFileSync(path.join(root,'src/variant_iter.rs'),'utf8');
if(!source.includes('let mut p: *mut libc::c_char')||!source.includes('&mut p,'))throw Error('glib out-pointer fix missing');
console.log(JSON.stringify({verified:true,upstreamCrateSha256:manifest.crateSha256,files:Object.keys(manifest.files).length,advisory:'RUSTSEC-2024-0429'}));
