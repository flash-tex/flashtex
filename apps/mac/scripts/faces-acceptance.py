#!/usr/bin/env python3
"""Headless app font/export acceptance; development only, no host TeX reads.

Uses binaries actually in --app, bounds every subprocess, preserves output and
all diagnostics. Unsupported source features are retained separately from font
packaging failures. --fixture-root defaults to fixtures/real-world.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import time

REPO=Path(__file__).resolve().parents[3]
# The missing-font-resource codes, derived from the one place they live
# (crates/render-pipeline/src/fontdiag.rs, via its generated manifest) rather
# than hand-copied here. A failure to read them is fatal by design: a gate
# that cannot state its own criteria must stop, not count zero.
sys.path.insert(0, str(REPO/'scripts'))
from font_diagnostics import codes as _font_codes  # noqa: E402
MISSING=set(_font_codes('substitution'))

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def missing(d):
    return d.get('code') in MISSING or (d.get('code')=='math_resource_profile' and d.get('message','').startswith('lmr'))

def run(argv,env,cwd,input=None):
    start=time.monotonic()
    try:
        p=subprocess.run(argv,input=input,capture_output=True,env=env,cwd=cwd,timeout=45)
        return p.returncode,p.stdout,p.stderr,round(time.monotonic()-start,3)
    except subprocess.TimeoutExpired as e:
        return None,e.stdout or b'',e.stderr or b'',round(time.monotonic()-start,3)

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--app',type=Path,required=True)
    ap.add_argument('--evidence',type=Path,required=True)
    ap.add_argument('--fixture-root',type=Path,default=REPO/'fixtures/real-world')
    ap.add_argument('--case',action='append',default=[])
    ap.add_argument('--require-optical',action='store_true',help='require emitted Roman8/Roman6 and refusal exporting a list after Roman8 removal')
    args=ap.parse_args()
    app=args.app.resolve();out=args.evidence.resolve();out.mkdir(parents=True,exist_ok=False)
    resources=app/'Contents/Resources'; fonts=resources/'Fonts'
    render=app/'Contents/MacOS/flashtex-render'; pdf=app/'Contents/MacOS/flashtex-pdf-exact'
    if not render.is_file() or not pdf.is_file():ap.error('both bundled producer and exact PDF executable are required')
    cases=args.case or ['hw1','lecture-notes','math-sheet','letter','unicode-accents','input-bibliography']
    fixtures=[]
    for name in cases:
        base=args.fixture_root/name
        entries=sorted(base.glob('*.tex'))
        entry=base/'main.tex' if (base/'main.tex').is_file() else entries[0] if len(entries)==1 else None
        if entry is None:ap.error('cannot identify entry for '+name)
        docs=[{'path':str(p.relative_to(base)),'text':p.read_text(encoding='utf-8')} for p in sorted(base.rglob('*.tex'))]
        fixtures.append((name,str(entry.relative_to(base)),docs))
    fixtures.append(('optical-12pt','main.tex',[{'path':'main.tex','text':r'\documentclass[12pt]{article}\begin{document}Optical $x_{1_{2}} + \frac{1}{2}$.\end{document}'}]))
    result={'app':str(app),'producer_sha256':sha(render),'pdf_sha256':sha(pdf),'host_tex_denied':True,
            'producer_component':json.loads((resources/'components.json').read_text()).get('render'),
            'scope':'Font packaging/export only. Unsupported compiler features and math profile limitations remain explicit.',
            'runs':[]}
    with tempfile.TemporaryDirectory(prefix='flashtex-faces-acceptance-') as tmp:
        work=Path(tmp);home=work/'home';home.mkdir()
        profile=out/'no-host-tex.sb';profile.write_text('(version 1)\n(allow default)\n'+''.join('(deny file-read* (subpath '+json.dumps(p)+'))\n' for p in ['/usr/local/texlive','/Library/TeX','/usr/share/texmf','/usr/share/texlive',str(REPO/'apps/mac/Fonts')]))
        env={'PATH':'/usr/bin:/bin','HOME':str(home),'FLASHTEX_FONT_DIRS':str(fonts),
             'FLASHTEX_TFM_DIRS':str(resources/'texmf/fonts/tfm/public/lm')}
        result['environment']=env
        all_faces=set()
        for revision,(name,entry,docs) in enumerate(fixtures,1):
            case=out/name;case.mkdir()
            request={'protocol_version':1,'id':name,'type':'compile','payload':{'project_id':'faces','revision':revision,'entry_path':entry,'documents':docs,'layout_capabilities':['display-list-v2']}}
            wire=(json.dumps(request,ensure_ascii=False)+'\n').encode('utf-8');(case/'request.jsonl').write_bytes(wire)
            display=case/'display.json'
            cmd=['/usr/bin/sandbox-exec','-f',str(profile),str(render),'--v2',str(display)]
            rc,stdout,stderr,seconds=run(cmd,env,home,wire)
            (case/'producer.jsonl').write_bytes(stdout);(case/'producer.stderr').write_bytes(stderr)
            row={'id':name,'argv':cmd,'exit':rc,'seconds':seconds,'request_sha256':sha(case/'request.jsonl'),'status':'failed'}
            try:
                replies=[json.loads(l) for l in stdout.splitlines() if l.strip()]
                compiles=[r for r in replies if r.get('type')=='compile_result']
                correlated=len(compiles)==1 and compiles[0].get('id')==name and compiles[0]['payload'].get('project_id')=='faces' and compiles[0]['payload'].get('revision')==revision
                row['correlated']=correlated
                row['diagnostics']=compiles[0]['payload'].get('diagnostics',[]) if compiles else []
                row['missing_fonts_metrics']=[d for d in row['diagnostics'] if missing(d)]
                row['compiler_status']=compiles[0]['payload'].get('status') if compiles else None
                if rc==0 and correlated and display.is_file():
                    frame=json.loads(display.read_text());payload=frame['payload']
                    if frame.get('type') != 'display_list' or frame.get('id') != name or payload.get('project_id') != 'faces' or payload.get('revision') != revision:
                        raise ValueError('uncorrelated display list')
                    row['faces']=[f['postscript_name'] for f in payload['fonts']];all_faces.update(row['faces'])
                    row['display_pages']=len(payload['pages']);row['display_sha256']=sha(display)
                    pth=case/'export.pdf'
                    argv=['/usr/bin/sandbox-exec','-f',str(profile),str(pdf),'from-v2',str(display),'--out',str(pth),'--font-dir',str(fonts)]
                    prc,pout,perr,pseconds=run(argv,env,home)
                    (case/'export.stdout').write_bytes(pout);(case/'export.stderr').write_bytes(perr)
                    row['export']={'argv':argv,'exit':prc,'seconds':pseconds,'pdf_sha256':sha(pth) if pth.is_file() else None}
                    row['status']='passed' if not row['missing_fonts_metrics'] and prc==0 and pth.is_file() else 'failed'
            except (ValueError,KeyError,TypeError) as e:
                row['parse_error']=str(e)
            result['runs'].append(row);print(name+': '+row['status'],flush=True)
            (out/'report.json').write_text(json.dumps(result,indent=2)+'\n')
        if args.require_optical:
            result['optical_faces_exercised']={'Roman8':'LMRoman8-Regular' in all_faces,'Roman6':'LMRoman6-Regular' in all_faces}
            # A valid emitted optical list references the original GIDs/hashes.
            # Removing its Roman8 file must refuse export, never substitute.
            import shutil
            removed=work/'removed-fonts';shutil.copytree(fonts,removed);(removed/'lmroman8-regular.otf').unlink()
            env_removed=dict(env,FLASHTEX_FONT_DIRS=str(removed))
            negative=out/'removed-roman8';negative.mkdir()
            display=out/'optical-12pt/display.json'
            argv=['/usr/bin/sandbox-exec','-f',str(profile),str(pdf),'from-v2',str(display),'--out',str(negative/'should-refuse.pdf'),'--font-dir',str(removed)]
            rc,stdout,stderr,seconds=run(argv,env_removed,home)
            (negative/'stdout.txt').write_bytes(stdout);(negative/'stderr.txt').write_bytes(stderr)
            result['removed_export']={'argv':argv,'exit':rc,'seconds':seconds,'refused':rc not in (0,None) and b'LMRoman8-Regular' in stdout+stderr, 'message':(stdout+stderr).decode('utf-8',errors='replace')}
    result['passed']=all(r['status']=='passed' for r in result['runs']) and (not args.require_optical or all(result['optical_faces_exercised'].values()) and result['removed_export']['refused'])
    (out/'report.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS' if result['passed'] else 'FAIL')
    return 0 if result['passed'] else 1

if __name__=='__main__': raise SystemExit(main())
