"""Pinned quantifier capture audit without compiler execution."""
import collections,gzip,hashlib,json,subprocess
from pathlib import Path
PEER='e8234e4e'
ROOT='crates/preview-controller/docs/handoffs/'
def read(n):return subprocess.check_output(['git','show',PEER+':'+ROOT+n])
def sha(b):return hashlib.sha256(b).hexdigest()
def audit():
 e=json.loads(read('hw1-logical-candidate/evidence.json'))
 for n,h in e['files'].items():assert sha(read('hw1-logical-candidate/'+n))==h
 assert sha(Path(e['binary_path']).read_bytes())==e['binary_sha256']
 req=json.loads(read('hw1-logical-candidate/request.jsonl'));out=json.loads(read('hw1-logical-candidate/response.jsonl'))
 source=req['payload']['documents'][0]['text'].encode();assert sha(source)==e['source_sha256']
 assert req['id']==out['id'] and out['type']=='compile_result'
 for field in ['project_id','revision']:assert req['payload'][field]==out['payload'][field]
 base=json.loads(gzip.decompress(read('hw1-starred-sections/base.jsonl.gz')))
 key=lambda x:json.dumps(x,sort_keys=True)
 old=collections.Counter(map(key,base['payload']['diagnostics']));new=collections.Counter(map(key,out['payload']['diagnostics']))
 assert len(out['payload']['diagnostics'])==112 and not(new-old)
 removed=[json.loads(s) for s,n in (old-new).items() for _ in range(n)];assert len(removed)==7
 spans=[];counts=collections.Counter()
 for page in out['payload']['pages']:
  for item in page['items']:
   if item.get('text') not in ['∨','⇒']:continue
   span=item['source'];symbol=item['text'];token={'∨':b'\\vee','⇒':b'\\Rightarrow'}[symbol]
   assert span['path']=='HW1.tex' and source[span['start_byte']:span['end_byte']]==token
   counts[symbol]+=1;spans.append(span)
 assert counts=={'∨':4,'⇒':3}
 assert collections.Counter(map(key,spans))==collections.Counter(key(d['source']) for d in removed)
 assert all(d['severity']=='error' and any(cmd in d['message'] for cmd in ['\\vee','\\Rightarrow']) for d in removed)
 for command,count in [('mid',4),('setminus',2),('Longrightarrow',1)]:
  assert sum(d['message']==f'\\{command} is not supported in math mode' for d in out['payload']['diagnostics'])==count
 return {'peer':PEER,'binary_sha256':e['binary_sha256'],'source_sha256':sha(source),'file_hashes':e['files'],'diagnostics':[119,112],'removed':removed,'symbol_counts':dict(counts),'spans':spans,'scope':'exact archived request/output and preserved executable checked; no rerun/PDF/native/combined adoption'}
if __name__=='__main__':print(json.dumps(audit(),indent=2,sort_keys=True))
