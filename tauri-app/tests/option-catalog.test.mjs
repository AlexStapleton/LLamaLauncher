import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseHelp, valuesFor, buildOverrides } from '../src/frontend/option-catalog.js';
import { impactFor } from '../src/frontend/option-impact.js';
const help = readFileSync(new URL('../src/frontend/server-help.txt', import.meta.url), 'utf8');
const catalog = parseHelp(help);
test('every installed help entry is represented, including wrapped headers', () => {
  const headers = help.split(/\r?\n/).filter(line => /^-{1,2}[A-Za-z]/.test(line));
  assert.equal(catalog.length, headers.length);
  assert.ok(catalog.length > 150);
  for (const option of catalog) { assert.ok(option.description); assert.ok(option.aliases.includes(option.flag)); }
  for (const flag of ['--models-dir', '--tools', '--mcp-servers-json', '--spec-default', '--reasoning-budget', '--control-vector-layer-range']) assert.ok(catalog.some(option => option.aliases.includes(flag)), flag);
});
test('flag pairs, enumerated values, and multiline descriptions remain usable', () => {
  const kv = catalog.find(option => option.flag === '--kv-offload');
  assert.ok(kv.aliases.includes('--no-kv-offload')); assert.equal(kv.syntax, '');
  const flash = catalog.find(option => option.flag === '--flash-attn');
  assert.deepEqual(flash.choices, ['on', 'off', 'auto']);
  const cache = catalog.find(option => option.flag === '--cache-type-k');
  assert.ok(cache.choices.includes('q4_0')); assert.ok(cache.choices.includes('f16'));
  const load = catalog.find(option => option.flag === '--load-mode');
  assert.ok(load.description.includes('force system to keep model in RAM'));
});
test('argument values preserve Windows paths and multi-value flags', () => {
  const lora = catalog.find(option => option.flag === '--lora');
  const path = 'D:\\Models & adapters\\my adapter.gguf';
  assert.deepEqual(valuesFor(lora, path), [path]);
  const range = catalog.find(option => option.flag === '--control-vector-layer-range');
  assert.deepEqual(valuesFor(range, '0 32'), ['0', '32']);
  assert.throws(() => valuesFor(range, '0'));
  assert.deepEqual(buildOverrides({ one: { flag: '--lora', values: [path], enabled: true }, two: { flag: '--verbose', values: [], enabled: false } }), [{flag:'--lora', values:[path]}]);
});
test('every option has separate speed, VRAM and RAM guidance', () => {
  for (const option of catalog) {
    const effects = impactFor(option);
    for (const key of ['speed', 'vram', 'ram']) assert.ok(effects[key]?.length > 20, `${option.flag}: ${key}`);
  }
});
test('multi-value options from different executable versions preserve paths with spaces', () => {
  const old = parseHelp('--lora-scaled FNAME SCALE  load adapter\n--control-vector-layer-range START END  layers');
  const current = parseHelp('--lora-scaled FNAME:SCALE,...  load adapters');
  assert.equal(old[0].arity,2);
  assert.deepEqual(valuesFor(old[0],'["/models/my adapter.gguf", "0.5"]'),['/models/my adapter.gguf','0.5']);
  assert.deepEqual(valuesFor(current[0],'/models/my adapter.gguf:0.5'),['/models/my adapter.gguf:0.5']);
  assert.throws(()=>valuesFor(old[0],'["one"]'));
});
