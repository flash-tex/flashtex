"""Read-only cumulative/text artifact audit; no compiler executions."""
import gzip,hashlib,json,subprocess,collections
from pathlib import Path
ROOT='crates/preview-controller/docs/handoffs/'
def read(peer,folder,name):return subprocess.check_output(['git','show',f'{peer}:{ROOT}{folder}/{name}'])
def sha(b):return hashlib.sha256(b).hexdigest()
def audit():
 final='be426cb8';opt='eaba97d5';f='hw1-current-compiler';o='hw1-text-runs'
 manifests={}
 for peer,folder in [(final,f),(opt,o)]:
  e=json.loads(read(peer,folder,'evidence.json'));manifests[folder]=e
  for name,h in e['files'].items():assert sha(read(peer,folder,name))==h
 e=manifests[f];oe=manifests[o]
 assert sha(Path(e['binary_path']).read_bytes())==e['binary_sha256']
 equivalence=json.loads(read(opt,o,'equivalence.json'))
 for side in ['before','after']:assert sha(Path(equivalence[side+'_binary']).read_bytes())==equivalence[side+'_sha256']
 req=read(final,f,'request.jsonl');response=read(final,f,'response.jsonl')
 before=gzip.decompress(read(opt,o,'before-response.jsonl.gz'));after=gzip.decompress(read(opt,o,'after-response.jsonl.gz'))
 assert before==after
 requests=[json.loads(line) for line in gzip.decompress(read(opt,o,'request.jsonl.gz')).splitlines()]
 replies=[json.loads(line) for line in after.splitlines()]
 assert len(requests)==len(replies)==207
 for request,reply in zip(requests,replies):
  assert request['id']==reply['id']
  for field in ['project_id','revision']:assert request['payload'][field]==reply['payload'][field]
 r=json.loads(req);result=json.loads(response)
 assert r['id']==result['id']
 for k in ['revision','project_id']:assert r['payload'][k]==result['payload'][k]
 source=r['payload']['documents'][0]['text'].encode()
 assert sha(source)=='f725e23897df3c9645bde1ceaab0876d78ea6ef4a44fd0e80d60a5161fac9d9e'
 diagnostics=result['payload']['diagnostics'];assert len(diagnostics)==e['hw1_diagnostics']==72
 title=[d for d in diagnostics if d['message'].startswith(('\\hfill ','\\normalfont '))]
 assert len(title)==14 and all(d['severity']=='error' for d in title)
 for command,count in [('mid',4),('setminus',2),('Longrightarrow',1)]:
  assert sum(d['message']==f'\\{command} is not supported in math mode' for d in diagnostics)==count
 assert e['source_hashes']['crates/compiler/src/math.rs']==oe['after_math_sha256']
 assert len(e['source_hashes'])==10 and e['replaces_all_prior_compiler_patches']
 return {'final_peer':final,'optimized_peer':opt,'source_sha256':sha(source),'binary_sha256':e['binary_sha256'],'before_after_response_sha256':sha(after),'final_response_sha256':sha(response),'diagnostics':dict(collections.Counter(d['message'] for d in diagnostics)),'checked_manifest_files':sum(len(x['files']) for x in manifests.values()),'owner_randomized_equivalence':equivalence,'scope':'all207 archived whole responses byte equal before/after with request correlation;final HW1 separate72diagnostic artifact, no replay; no visual fidelity claim or whole-PDF byte acceptance'}
if __name__=='__main__':print(json.dumps(audit(),indent=2,sort_keys=True))
