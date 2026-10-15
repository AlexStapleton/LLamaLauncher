import type {Artifact,JobReport} from './native-types';
export interface RuntimeSelection {label?:string;version?:string;tag?:string;backend?:string;reason?:string}
export interface DownloadedModel {model:string;model_id:string;name?:string}
export interface JobResult {
  [key:string]:unknown;
  artifact?:Artifact;selection?:RuntimeSelection;models?:DownloadedModel[];
  model?:string;model_id?:string;directory?:string;
  text?:string;first_token_seconds?:number;seconds?:number;tokens?:number;tokens_per_second?:number;
  code?:string;
}
function object(value:unknown):Record<string,unknown>{if(!value||typeof value!=='object'||Array.isArray(value))throw Error('Invalid job result');return value as Record<string,unknown>;}
export function decodeJobReport(value:unknown):JobReport {
  const report=object(value);for(const key of ['id','kind','phase','message'])if(typeof report[key]!=='string')throw Error(`Invalid job ${key}`);
  if(!['running','complete','cancelled','failed'].includes(String(report.phase)))throw Error('Invalid job phase');
  for(const key of ['completed','total'])if(typeof report[key]!=='number'||!Number.isFinite(report[key])||Number(report[key])<0)throw Error(`Invalid job ${key}`);
  const result=object(report.result);
  for(const key of ['text','model','model_id','directory','code'])if(result[key]!==undefined&&typeof result[key]!=='string')throw Error(`Invalid job result ${key}`);
  for(const key of ['first_token_seconds','seconds','tokens','tokens_per_second'])if(result[key]!==undefined&&(typeof result[key]!=='number'||!Number.isFinite(result[key])||Number(result[key])<0))throw Error(`Invalid job measurement ${key}`);
  if(result.selection!==undefined){const selection=object(result.selection);for(const key of ['label','version','tag','backend','reason'])if(selection[key]!==undefined&&typeof selection[key]!=='string')throw Error(`Invalid runtime selection ${key}`);}
  if(result.artifact!==undefined){const artifact=object(result.artifact);for(const key of ['id','kind','name','path','version'])if(typeof artifact[key]!=='string')throw Error(`Invalid runtime artifact ${key}`);if(typeof artifact.managed!=='boolean')throw Error('Invalid artifact ownership');object(artifact.details);}
  if(result.models!==undefined){if(!Array.isArray(result.models)||result.models.length>1024)throw Error('Invalid downloaded models');for(const value of result.models){const model=object(value);if(typeof model.model!=='string'||typeof model.model_id!=='string')throw Error('Invalid downloaded model');}}
  return report as unknown as JobReport;
}
