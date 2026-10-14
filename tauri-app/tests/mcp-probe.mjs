// Read-only MCP connectivity check: initialize, initialized notification, tools/list. No tools are invoked.
const base = process.env.LLAMA_MCP_URL;
const token = process.env.LLAMA_MCP_TOKEN;
if (!base || !token) throw new Error('Supply LLAMA_MCP_URL and LLAMA_MCP_TOKEN.');
async function probe(url) {
  let session, protocol='2025-11-25';
  async function rpc(message) {
    const headers={'Content-Type':'application/json',Accept:'application/json, text/event-stream',Authorization:`Bearer ${token}`};
    if(session)headers['Mcp-Session-Id']=session;
    if(message.method!=='initialize')headers['MCP-Protocol-Version']=protocol;
    let response;
    for(let redirects=0;redirects<4;redirects++) {
      response=await fetch(url,{method:'POST',headers,body:JSON.stringify(message),signal:AbortSignal.timeout(12000),redirect:'manual'});
      if(response.status<300||response.status>=400)break;
      const redirected=new URL(response.headers.get('location'),url);
      if(redirected.origin!==new URL(url).origin)throw new Error('Endpoint redirects to a different origin.');
      url=redirected.href;await response.body?.cancel();
    }
    if(!response.ok)throw new Error(`HTTP ${response.status}`);
    session=response.headers.get('mcp-session-id')||session;
    if(message.id===undefined){await response.body?.cancel();return;}
    let reply;
    if((response.headers.get('content-type')||'').includes('text/event-stream')){
      const reader=response.body.getReader(),decoder=new TextDecoder();let buffered='',data=[];
      outer:while(true){const{done,value}=await reader.read();if(done)break;buffered+=decoder.decode(value,{stream:true});let index;
        while((index=buffered.indexOf('\n'))>=0){const line=buffered.slice(0,index).replace(/\r$/,'');buffered=buffered.slice(index+1);
          if(line.startsWith('data:'))data.push(line.slice(5).trimStart());
          if(!line&&data.length){const parsed=JSON.parse(data.join('\n'));data=[];if(parsed.id===message.id){reply=parsed;await reader.cancel();break outer;}}
        }
      }
    }else reply=await response.json();
    if(reply?.error)throw new Error(`JSON-RPC error ${reply.error.code}`);
    if(reply?.id!==message.id||!reply.result)throw new Error('No matching JSON-RPC result.');
    return reply.result;
  }
  const initialized=await rpc({jsonrpc:'2.0',id:1,method:'initialize',params:{protocolVersion:protocol,capabilities:{},clientInfo:{name:'llama-server-configurator',version:'1.0.0'}}});
  protocol=initialized.protocolVersion;
  await rpc({jsonrpc:'2.0',method:'notifications/initialized'});
  const tools=[];let cursor,id=2;
  do{const result=await rpc({jsonrpc:'2.0',id:id++,method:'tools/list',params:cursor?{cursor}:{}});tools.push(...(result.tools||[]));cursor=result.nextCursor;if(id>102)throw new Error('Too many tool pages.');}while(cursor);
  if(session){const headers={Authorization:`Bearer ${token}`,'Mcp-Session-Id':session,'MCP-Protocol-Version':protocol};await fetch(url,{method:'DELETE',headers,signal:AbortSignal.timeout(5000)}).catch(()=>{});}
  return{endpoint:url,protocol,server:initialized.serverInfo?.name,tools:tools.map(tool=>({name:tool.name,description:tool.description||''})),count:tools.length};
}
let result;
try{result=await probe(base);}catch(error){if(!/^HTTP (404|405)$/.test(error.message))throw error;result=await probe(new URL('/mcp',base).href);}
console.log(JSON.stringify(result));
