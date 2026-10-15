// Parse the installed executable's help, rather than assume a particular llama.cpp version.
import {guideFor} from './option-guide.js';
/** @param {string} text @returns {import('./option-types').ServerOption[]} */
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
      const matches = [...header.matchAll(/(?:^|[ ,])(--?[A-Za-z][\w.-]*)/g)];
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
    const guide = guideFor(option);
    const category = guide.category;
    const arity = /^[A-Z][A-Z_]*(?: [A-Z][A-Z_]*)+$/.test(option.syntax) ? option.syntax.split(' ').length : option.syntax ? 1 : 0;
    return { ...option, description, choices, arity, defaultValue: defaultMatch?.[1]?.trim() || '', category, guide, searchText: `${option.aliases.join(' ')} ${description} ${guide.title} ${guide.summary} ${category} ${guide.group}`.toLowerCase() };
  });
}

export function categoryFor(flag) { return guideFor({flag}).category; }

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

export function optionGroup(option) {return (option.guide || guideFor(option)).group;}
