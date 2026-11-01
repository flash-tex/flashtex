#!/usr/bin/env python3
"""Run installed TeX only as a bounded test oracle; never install or generate PDFs."""
import argparse, hashlib, json, pathlib, shutil, subprocess
parser=argparse.ArgumentParser();parser.add_argument('--output',required=True);args=parser.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]
source=root/'fixtures/boundary-oracle/cases.json';manifest=json.loads(source.read_text())
output=pathlib.Path(args.output);output.mkdir(parents=True,exist_ok=False)
engine=shutil.which('tex') or shutil.which('pdftex');convert=shutil.which('tftopl')
evidence={'schema_version':1,'manifest_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'engine':engine,'tftopl':convert,'status':'pending','cases':[]}
if engine and convert:
    evidence['engine_version']=subprocess.check_output([engine,'--version'],timeout=10).decode()
    evidence['engine_sha256']=hashlib.sha256(pathlib.Path(engine).read_bytes()).hexdigest()
    evidence['tftopl_sha256']=hashlib.sha256(pathlib.Path(convert).read_bytes()).hexdigest()
    evidence['tftopl_version']=subprocess.check_output([convert,'--version'],timeout=10).decode()
    for case in manifest['cases']:
        directory=output/case['id'];directory.mkdir();font=bytes.fromhex(case['tfm_hex'])
        assert len(font)<=128*1024 and hashlib.sha256(font).hexdigest()==case['tfm_sha256']
        (directory/'probe.tfm').write_bytes(font)
        convert_run=subprocess.run([convert,'probe.tfm','probe.pl'],cwd=directory,capture_output=True,timeout=20)
        (directory/'tftopl.stdout').write_bytes(convert_run.stdout+convert_run.stderr)
        codes=''.join('\\char%d '%code for code in case['input_codes'])
        tex='\\nonstopmode\n\\tracingonline=1 \\showboxbreadth=100 \\showboxdepth=10\n\\font\\probe=probe at 10pt\n\\setbox0=\\hbox{\\probe '+codes+'}\n\\showbox0\n\\end\n'
        (directory/'oracle.tex').write_text(tex)
        run=subprocess.run([engine,'-interaction=nonstopmode','-no-shell-escape','oracle.tex'],cwd=directory,capture_output=True,timeout=20)
        (directory/'engine.stdout').write_bytes(run.stdout+run.stderr)
        evidence['cases'].append({'id':case['id'],'tfm_sha256':case['tfm_sha256'],'engine_exit':run.returncode,'tftopl_exit':convert_run.returncode,'status':'raw_node_evidence_requires_adjudication','files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.iterdir() if p.is_file()}})
    evidence['status']='raw_evidence_captured_not_parity'
else:
    evidence['reason']='Existing tex/pdftex and tftopl executables required; no installation attempted.'
(output/'result.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(output/'result.json')
