#!/usr/bin/env python3
"""Verify an unchanged git archive, rebuild offline, then replay actual transport.
No download/install, font substitution, source metadata repair or visual claim.
"""
import argparse, hashlib, io, json, os, pathlib, subprocess, tarfile
p=argparse.ArgumentParser();p.add_argument('--producer-tree',required=True);p.add_argument('--font-dir',required=True);p.add_argument('--tfm-dir',required=True);p.add_argument('--output',required=True);p.add_argument('--producer-sha',default='65dbe7d');a=p.parse_args()
repo=pathlib.Path(__file__).resolve().parents[3];tree=pathlib.Path(a.producer_tree).resolve();out=pathlib.Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
def run(argv,**kw):return subprocess.run(argv,check=True,capture_output=True,**kw)
def sha(b):return hashlib.sha256(b).hexdigest()
exact=run(['git','rev-parse',a.producer_sha],cwd=repo).stdout.decode().strip()
archive=run(['git','archive',exact,'crates/render-pipeline'],cwd=repo).stdout
with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
 files=[m for m in tar if m.isfile()]
 expected={m.name for m in files}
 extras=[str(f.relative_to(tree)) for f in (tree/'crates/render-pipeline').rglob('*') if f.is_file() and 'target' not in f.relative_to(tree).parts and str(f.relative_to(tree)) not in expected]
 if extras:raise RuntimeError('untracked build inputs: '+str(extras))
 for member in files:
  if (tree/member.name).read_bytes()!=tar.extractfile(member).read():raise RuntimeError('source mismatch: '+member.name)
cargo=str(pathlib.Path.home()/'.cargo/bin/cargo');manifest=tree/'crates/render-pipeline/Cargo.toml'
build=[cargo,'build','--offline','--manifest-path',str(manifest),'--bin','flashtex-render','--quiet'];build_env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_INCREMENTAL='0');built=subprocess.run(build,env=build_env,capture_output=True);(out/'build.log').write_bytes(built.stdout+built.stderr);built.check_returncode()
binary=manifest.parent/'target/debug/flashtex-render'
assets=[]
for folder in [pathlib.Path(a.font_dir),pathlib.Path(a.tfm_dir)]:
 for f in sorted(folder.iterdir()):
  if f.is_file():assets.append({'path':str(f.resolve()),'sha256':sha(f.read_bytes()),'bytes':f.stat().st_size})
license_path=pathlib.Path(a.tfm_dir).parents[3]/'doc/fonts/lm/GUST-FONT-LICENSE.TXT'
assets.append({'path':str(license_path),'sha256':sha(license_path.read_bytes()),'bytes':license_path.stat().st_size})
env=dict(os.environ,FLASHTEX_FONT_DIRS=a.font_dir,FLASHTEX_TFM_DIRS=a.tfm_dir,FLASHTEX_REPLAY_PRODUCER=str(binary),FLASHTEX_REPLAY_OUTPUT=str(out/'runtime.json'))
test=[cargo,'test','--offline','--manifest-path',str(repo/'crates/document-runtime/Cargo.toml'),'--test','real_display_producer','--','--ignored']
result=subprocess.run(test,env=env,capture_output=True);(out/'test.log').write_bytes(result.stdout+result.stderr);result.check_returncode()
fixture_path=repo/'crates/document-runtime/fixtures/display-producer-request.json'
fixture=json.loads(fixture_path.read_text());raw_evidence=[]
for mode,limit in [('requested',None),('legacy',None),('declined','1500'),('failed','1')]:
 request=json.loads(json.dumps(fixture))
 if mode=='legacy':request['payload'].pop('layout_capabilities')
 wire=(json.dumps(request)+'\n').encode();case_env=dict(env);case_env.pop('FLASHTEX_MAX_REPLY_BYTES',None)
 if limit is not None:case_env['FLASHTEX_MAX_REPLY_BYTES']=limit
 raw=run([str(binary)],input=wire,env=case_env,timeout=20)
 (out/(mode+'.request.jsonl')).write_bytes(wire);(out/(mode+'.stdout.jsonl')).write_bytes(raw.stdout);(out/(mode+'.stderr')).write_bytes(raw.stderr)
 raw_evidence.append({'mode':mode,'request_sha256':sha(wire),'stdout_sha256':sha(raw.stdout),'stderr_sha256':sha(raw.stderr),'reply_limit':limit})
incremental_fixture=repo/'crates/document-runtime/fixtures/display-incremental-requests.json'
incremental=json.loads(incremental_fixture.read_text());incremental_raw=[]
for name,limit in [('requested',None),('bounded','2500')]:
 case_env=dict(env);case_env.pop('FLASHTEX_MAX_REPLY_BYTES',None)
 if limit is not None:case_env['FLASHTEX_MAX_REPLY_BYTES']=limit
 inputs=[(json.dumps(case['request'])+'\n').encode() for case in incremental]
 fresh=b''.join(run([str(binary)],input=wire,env=case_env,timeout=20).stdout for wire in inputs)
 persistent=run([str(binary)],input=b''.join(inputs),env=case_env,timeout=30).stdout
 (out/('incremental-'+name+'.fresh.jsonl')).write_bytes(fresh);(out/('incremental-'+name+'.persistent.jsonl')).write_bytes(persistent)
 if fresh!=persistent:raise RuntimeError('fresh/persistent raw byte mismatch: '+name)
 incremental_raw.append({'mode':name,'reply_limit':limit,'byte_equal':True,'sha256':sha(fresh),'bytes':len(fresh)})
report=json.loads((out/'runtime.json').read_text());font_hashes={entry['sha256'] for entry in assets}
for case in report:
 raw_lines=[json.loads(line) for line in (out/(case['mode']+'.stdout.jsonl')).read_bytes().splitlines()]
 if raw_lines[0]!=case['result'] or (raw_lines[1] if len(raw_lines)>1 else None)!=case['candidate']:raise RuntimeError('direct/runtime response mismatch')
 candidate=case['candidate']
 if candidate:
  for font in candidate['payload']['fonts']:
   if font['sha256'] not in font_hashes:raise RuntimeError('producer font raw SHA not found in supplied assets')
evidence={'producer_sha':exact,'incremental_raw':incremental_raw,'incremental_fixture_sha256':sha(incremental_fixture.read_bytes()),'incremental_evidence_sha256':sha((out/'incremental.json').read_bytes()),'raw_cases':raw_evidence,'fixture_sha256':sha(fixture_path.read_bytes()),'archive_sha256':sha(archive),'verified_source_files':len(files),'binary_sha256':sha(binary.read_bytes()),'build_command':build,'build_environment':{'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_INCREMENTAL':'0'},'test_command':test,'assets':assets,'runtime_evidence_sha256':sha((out/'runtime.json').read_bytes()),'lifecycle_evidence_sha256':sha((out/'lifecycle.json').read_bytes()),'rendering_validation':'not performed','native_latency':'not measured'}
(out/'provenance.json').write_text(json.dumps(evidence,indent=2)+'\n');print(out)
