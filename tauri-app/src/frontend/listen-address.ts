import type {Config} from './native-types';
export function connectionAddress(config:Pick<Config,'host'|'port'|'argument_overrides'|'additional_args'>):string {
  function option(flag:string){
    let value=config.argument_overrides.filter(a=>a.flag===flag).at(-1)?.values[0];
    for(let i=0;i<config.additional_args.length;i++){
      const arg=config.additional_args[i];if(arg===flag)value=config.additional_args[++i];else if(arg.startsWith(flag+'='))value=arg.slice(flag.length+1);
    }return value;
  }
  if(config.host.endsWith('.sock'))return `Unix socket: ${config.host}`;
  const scheme=option('--ssl-key-file')!==undefined?'https':'http';
  const prefix=(option('--api-prefix')||'').replace(/\/$/,'');
  return config.host.split(',').map(value=>{
    const host=value.trim();
    const address=host==='0.0.0.0'?'<LAN IP>':host==='::'?'[<IPv6 address>]':host.includes(':')?`[${host}]`:host;
    return `${scheme}://${address}:${config.port}${prefix}`;
  }).join(' · ');
}
