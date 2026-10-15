import test from 'node:test';import assert from 'node:assert/strict';import {connectionAddress} from '../src/frontend/listen-address.ts';
const base={host:'127.0.0.1',port:8080,argument_overrides:[],additional_args:[]};
test('client addresses distinguish wildcard binding from usable network URLs',()=>{
  assert.equal(connectionAddress(base),'http://127.0.0.1:8080');assert.equal(connectionAddress({...base,host:'0.0.0.0'}),'http://<LAN IP>:8080');assert.equal(connectionAddress({...base,host:'192.168.1.50',port:9000}),'http://192.168.1.50:9000');assert.equal(connectionAddress({...base,host:'::1'}),'http://[::1]:8080');assert.equal(connectionAddress({...base,host:'::'}),'http://[<IPv6 address>]:8080');
});
test('connection address includes configured TLS and API prefix',()=>{
  const config={...base,host:'10.0.0.5',argument_overrides:[{flag:'--ssl-key-file',values:['key.pem']},{flag:'--api-prefix',values:['/old']}],additional_args:['--api-prefix=/api/']};assert.equal(connectionAddress(config),'https://10.0.0.5:8080/api');assert.equal(connectionAddress({...base,host:'/tmp/llama.sock'}),'Unix socket: /tmp/llama.sock');
});
