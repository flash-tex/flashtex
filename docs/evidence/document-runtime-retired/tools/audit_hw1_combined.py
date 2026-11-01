"""Independent combined artifact audit; no compiler execution."""
import collections,gzip,hashlib,json,subprocess
from pathlib import Path
PEER='e90b75a3'
BASE='crates/preview-controller/docs/handoffs/'
def read(n):return subprocess.check_output(['git','show',PEER+':'+BASE+n])
def sha(b):return hashlib.sha256(b).hexdigest()
def audit():
 e=json.loads(read('hw1-membership/evidence.json'))
 for n,h in e['files'].items():assert sha(read('hw1-membership/'+n))==h
 p=json.loads(read('hw1-membership/combined-evidence.json'))
 binary=Path('/home/natkarri/flashtex-captures/hw1-membership-1c02a2bc/combined-compiler-preserved')
 assert sha(binary.read_bytes())==p['binary_sha256']
 out=json.loads(read('hw1-membership/hw1-combined.jsonl'))
 source=json.loads(gzip.decompress(read('hw1-starred-sections/request.jsonl.gz')))['payload']['documents'][0]['text'].encode()
 assert sha(source)==p['source_sha256']
 diagnostics=out['payload']['diagnostics'];assert len(diagnostics)==p['diagnostics']==100
 counts=collections.Counter(d['message'] for d in diagnostics);assert dict(counts)==p['remaining_commands']
 title=[d for d in diagnostics if d['message'].startswith(('\\hfill ','\\normalfont '))]
 assert len(title)==14 and all(d['severity']=='error' for d in title)
 items=[i for page in out['payload']['pages'] for i in page['items'] if i.get('text')=='∈']
 assert len(items)==p['membership_items']==12
 for i in items:
  span=i['source'];assert span['path']=='HW1.tex' and source[span['start_byte']:span['end_byte']]==b'\\in'
 base=json.loads(gzip.decompress(read('hw1-starred-sections/base.jsonl.gz')))
 key=lambda d:json.dumps(d,sort_keys=True)
 old=collections.Counter(map(key,base['payload']['diagnostics']));new=collections.Counter(map(key,diagnostics))
 return {'peer':PEER,'source_sha256':sha(source),'combined_output_sha256':sha(read('hw1-membership/hw1-combined.jsonl')),'recorded_binary_sha256':p['binary_sha256'],'diagnostics':100,'title_errors':title,'membership_items':12,'multiset_removed':[json.loads(d) for d,n in (old-new).items() for _ in range(n)],'multiset_added':[json.loads(d) for d,n in (new-old).items() for _ in range(n)],'scope':'preserved binary independently rehashed; exact combined stdin not archived; net19 reduction is not all-diagnostics subset or PDF parity'}
if __name__=='__main__':print(json.dumps(audit(),indent=2,sort_keys=True))
