// Parse the installed executable's help, rather than assume a particular llama.cpp version.
export function parseHelp(text) {
  const options = [];
  let current = null;
  let section = 'Common';
  for (const raw of text.replace(/\x1b\[[0-9;]*m/g, '').split(/\r?\n/)) {
    const line = raw.trimEnd();
    if (/^-----/.test(line)) { section = line.replace(/-/g, '').trim(); continue; }
    if (/^-{1,2}[A-Za-z]/.test(line)) {
      const split = line.match(/^(.*?)\s{2,}([A-Za-z].*)$/);
      const header = split ? split[1] : line;
      const matches = [...header.matchAll(/(?:^|[ ,])(--?[A-Za-z][\w-]*)/g)];
      const aliases = matches.map(m => m[1]);
      if (!aliases.length) continue;
      const last = matches.at(-1);
      const syntax = header.slice(last.index + last[0].length).trim();
      current = { flag: aliases.find(a => a.startsWith('--') && !a.startsWith('--no-')) || aliases[0], aliases, syntax, section, description: split ? split[2] : '' };
      options.push(current);
    } else if (current && line.trim()) current.description += ` ${line.trim()}`;
  }
  return options.map(option => {
    const explicit = option.syntax.match(/[\[{<]([^\]}>]+)[\]}>]/);
    const list = option.description.match(/allowed values:\s*([^()]+)/i);
    let choices = explicit && /[|,]/.test(explicit[1]) && !explicit[1].includes('..') ? explicit[1].split(/[|,]/) : list ? list[1].split(',') : [];
    choices = choices.map(c => c.trim()).filter(c => c && !c.includes(' ') && !c.includes('/'));
    const defaultMatch = option.description.match(/\(default:\s*([^)]*)\)/i);
    const description = option.description.replace(/\s+/g, ' ').trim();
    const category = categoryFor(option.flag);
    const arity = /^[A-Z][A-Z_]*(?: [A-Z][A-Z_]*)+$/.test(option.syntax) ? option.syntax.split(' ').length : option.syntax ? 1 : 0;
    return { ...option, description, choices, arity, defaultValue: defaultMatch?.[1]?.trim() || '', category, searchText: `${option.aliases.join(' ')} ${description} ${category}`.toLowerCase() };
  });
}

export function categoryFor(flag) {
  if (/^--(help|usage|version|cache-list|completion-bash|list-devices)$/.test(flag)) return 'Utilities';
  if (/gpu|tensor-split|split-mode|device|offload|flash-attn|cache-type|cpu-moe|cpu-ffn|moe-cache|^--fit/.test(flag)) return 'GPU & memory';
  if (/threads|cpu-|prio|poll|batch-size|numa|parallel|cont-batching|^--perf/.test(flag)) return 'Performance';
  if (/temp|top-|min-p|typical|repeat|penalty|mirostat|dry-|xtc-|sampler|seed|adaptive|logit|ignore-eos|predict/.test(flag)) return 'Sampling';
  if (/tools|mcp-|agent/.test(flag)) return 'Tools & integrations';
  if (/host|port|cors|api-|ssl-|timeout|sse-|metrics|props|slots|^--(ui|webui|embedding|embeddings|rerank|reranking|path)$/.test(flag)) return 'API & server';
  if (/log-|verbose|verbosity/.test(flag)) return 'Logging';
  if (/spec-|draft|lookup-cache/.test(flag)) return 'Speculative decoding';
  if (/model|hf-|docker|lora|control-vector|mmproj|media|image|video|fim-|gpt-oss|vision-|embd-/.test(flag)) return 'Models & adapters';
  if (/ctx|context|rope|yarn|keep|swa|cache-|kv-|load-mode|lazy-mode|repack|no-host|sleep/.test(flag)) return 'Context & cache';
  if (/jinja|chat-|reasoning|grammar|json-schema|prefill|escape|reverse-prompt|special/.test(flag)) return 'Templates & output';
  return 'Advanced';
}

export function valuesFor(option, text) {
  if (!option.syntax) return [];
  const arity = option.arity || (/^[A-Z][A-Z_]*(?: [A-Z][A-Z_]*)+$/.test(option.syntax) ? option.syntax.split(' ').length : 1);
  if (arity > 1) {
    const values = text.trim().startsWith('[') ? JSON.parse(text) : text.trim().split(/\s+/);
    if (!Array.isArray(values) || values.length !== arity || values.some(value=>typeof value!=='string'||!value.length)) throw new Error(`${option.flag} needs ${arity} values (${option.syntax}). Use a JSON string array for paths containing spaces.`);
    return values;
  }
  if (!text.trim()) throw new Error(`Enter a value for ${option.flag}, or turn off Override.`);
  return [text];
}

export function buildOverrides(settings) {
  return Object.values(settings).filter(setting => setting.enabled).map(({ flag, values }) => ({ flag, values }));
}
export function displayValues(option, values) {
  return option.arity > 1 && values.some(value=>/\s/.test(value)) ? JSON.stringify(values) : values.join(' ');
}

export function optionGroup(option, tab) {
  const flag=option.flag;
  if(tab==='performance') {
    if(/^--threads(?:-batch)?$/.test(flag))return 'CPU threads';
    if(/batch-size|cont-batching|parallel/.test(flag))return 'Batching & concurrency';
    if(/ctx-size|load-mode|lazy-mode|cache-ram/.test(flag))return 'Context & model loading';
    if(/cpu-mask|cpu-range|cpu-strict|numa/.test(flag))return 'CPU placement & affinity';
    if(/prio|poll/.test(flag))return 'Scheduling & polling';
    return 'Diagnostics';
  }
  if(tab==='gpu') {
    if(/cache-type/.test(flag))return 'KV cache precision';
    if(/tensor-split|split-mode|main-gpu/.test(flag))return 'Multi-GPU allocation';
    if(/flash-attn|^--fit/.test(flag))return 'Attention & memory fitting';
    return 'Devices & offloading';
  }
  return option.category;
}
