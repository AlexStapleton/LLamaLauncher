import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
test('typed controls match the HTML element contract',()=>{
  const html=fs.readFileSync(new URL('../src/frontend/index.html',import.meta.url),'utf8');
  const contract=fs.readFileSync(new URL('../src/frontend/dom.ts',import.meta.url),'utf8');
  const tags={input:'HTMLInputElement',select:'HTMLSelectElement',textarea:'HTMLTextAreaElement',button:'HTMLButtonElement'};
  for(const match of html.matchAll(/<([a-z]+)\b[^>]*\bid="([^"]+)"[^>]*>/g)){
    const declared=contract.match(new RegExp(`'${match[2]}': (\\w+)`));assert.ok(declared,`Missing typed control ${match[2]}`);
    if(tags[match[1]])assert.equal(declared[1],tags[match[1]],match[2]);
  }
});
