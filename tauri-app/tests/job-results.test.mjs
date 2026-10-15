import test from 'node:test';import assert from 'node:assert/strict';import {decodeJobReport} from '../src/frontend/job-results.ts';
const report={id:'fixture',kind:'inference',phase:'complete',message:'',completed:0,total:0,result:{text:'Fixture response',tokens:1,seconds:0.1,tokens_per_second:10}};
test('typed job boundary accepts valid completion and rejects malformed progress/measurements',()=>{
  assert.equal(decodeJobReport(report).result.tokens,1);
  for(const invalid of [{...report,completed:-1},{...report,phase:'unknown'},{...report,result:{tokens:'1'}},{...report,result:{seconds:NaN}},{...report,result:{selection:{backend:1}}},{...report,result:{models:[{model:'file.gguf'}]}},{...report,result:{artifact:{path:'file.exe'}}}])assert.throws(()=>decodeJobReport(invalid));
});
