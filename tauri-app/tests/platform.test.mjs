import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizePath, resolveModelPath, parentPath, baseName, commandText } from '../src/frontend/platform.ts';
test('Linux model paths retain case and literal backslashes',()=>{
  assert.notEqual(normalizePath('/models/Model.gguf',false),normalizePath('/models/model.gguf',false));
  assert.equal(normalizePath('/models/a\\b.gguf',false),'/models/a\\b.gguf');
  assert.equal(normalizePath('C:\\Models\\Model.gguf',true),'c:/models/model.gguf');
  assert.equal(resolveModelPath('\\model.gguf','/models',false),'/models/\\model.gguf');
  assert.equal(resolveModelPath('model.gguf','C:\\',true),'c:/model.gguf');
  assert.equal(parentPath('/models/a\\b.gguf',false),'/models');
  assert.equal(baseName('/models/a\\b.gguf',false),'a\\b.gguf');
});
test('command previews quote shell metacharacters without executing them',()=>{
  const args=['/opt/llama-server',"/models/O'Brien $model.gguf"];
  assert.equal(commandText(args,false),"'/opt/llama-server' '/models/O'\\''Brien $model.gguf'");
  assert.equal(commandText(args,true),"& '/opt/llama-server' '/models/O''Brien $model.gguf'");
});
