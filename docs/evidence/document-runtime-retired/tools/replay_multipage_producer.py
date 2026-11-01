#!/usr/bin/env python3
"""Bounded correctness replay reusing a verified producer; no build or cap changes."""
import argparse,gzip,hashlib,json,os,pathlib,subprocess
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--provenance',required=True);p.add_argument('--font-dir',required=True);p.add_argument('--tfm-dir',required=True);p.add_argument('--output',required=True);a=p.parse_args()
sha=lambda b:hashlib.sha256(b).hexdigest()
repo=pathlib.Path(__file__).resolve().parents[3];out=pathlib.Path(a.output).resolve();out.mkdir(exist_ok=False,parents=True)
proof=pathlib.Path(a.provenance);e=json.loads(proof.read_text());assert sha(pathlib.Path(a.binary).read_bytes())==e['binary_sha256']
for asset in e['assets']:assert sha(pathlib.Path(asset['path']).read_bytes())==asset['sha256']
fixture=repo/'crates/document-runtime/fixtures/display-multipage-requests.json';cases=json.loads(fixture.read_text());assert len(cases)<=3
base=dict(os.environ,FLASHTEX_REPLAY_PRODUCER=a.binary,FLASHTEX_FONT_DIRS=a.font_dir,FLASHTEX_TFM_DIRS=a.tfm_dir,FLASHTEX_MULTIPAGE_REPLAY='1',FLASHTEX_REPLAY_OUTPUT=str(out/'runtime.json'))
command=[str(pathlib.Path.home()/'.cargo/bin/cargo'),'test','--offline','--manifest-path',str(repo/'crates/document-runtime/Cargo.toml'),'--test','real_display_producer','actual_incremental_producer_fresh','--','--ignored']
test=subprocess.run(command,env=base,capture_output=True);(out/'test.log').write_bytes(test.stdout+test.stderr);test.check_returncode()
report=[]
for mode,limit in [('default',None),('bounded','2000000')]:
 env=dict(base);env.pop('FLASHTEX_MAX_REPLY_BYTES',None)
 if limit:env['FLASHTEX_MAX_REPLY_BYTES']=limit
 inputs=[(json.dumps(c['request'])+'\n').encode() for c in cases]
 def run(wire):return subprocess.run([a.binary],input=wire,env=env,capture_output=True,check=True,timeout=30).stdout
 fresh=b''.join(run(wire) for wire in inputs);persistent=run(b''.join(inputs))
 assert fresh==persistent
 compressed=gzip.compress(persistent,mtime=0);(out/(mode+'.stdout.jsonl.gz')).write_bytes(compressed)
 frames=[]
 for line in persistent.splitlines():
  v=json.loads(line);pages=v['payload']['pages'];assert 25<=len(pages)<=29
  frames.append({'request_id':v['id'],'type':v['type'],'bytes_with_newline':len(line)+1,'sha256':sha(line+b'\n'),'pages':len(pages),'page_item_counts':[len(p['items']) for p in pages],'status':v['payload']['status'],'diagnostics':v['payload']['diagnostics']})
 assert len(frames)==3 # all v2 siblings explicitly declined, no truncation disguised as delivery
 report.append({'mode':mode,'reply_limit':limit,'fresh_persistent_bytes_equal':True,'raw_sha256':sha(persistent),'gzip_sha256':sha(compressed),'frames':frames})
summary={'producer_sha':e['producer_sha'],'binary_sha256':e['binary_sha256'],'asset_provenance_sha256':sha(proof.read_bytes()),'fixture_sha256':sha(fixture.read_bytes()),'test_command':command,'cases':report,'runtime_summary_sha256':sha((out/'incremental.json').read_bytes()),'scope':'complete v1 equality, explicit v2 decline; no large-v2 acceptance or performance claim'}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(out)
