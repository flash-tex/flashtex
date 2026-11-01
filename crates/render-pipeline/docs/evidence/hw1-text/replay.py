import pathlib, subprocess, json, os, hashlib, sys
S = pathlib.Path(__file__).parent
EV = S.parent / 'hw1/crates/rendering-core/docs/handoffs/hw1-text-producer/actual-candidate/cases'
stage = S / 'stage'
cases = ['comment', 'escaped', 'five', 'ligature', 'scripts', 'space']
which = sys.argv[1]  # before | after
out = S / 'out' / which
out.mkdir(parents=True, exist_ok=True)
binary = S / which / 'target/release/flashtex-render'
env = dict(os.environ, FLASHTEX_TFM_DIRS=str(stage / 'metrics'), FLASHTEX_FONT_DIRS=str(stage / 'fonts'))
env.pop('FLASHTEX_LM_DIR', None)
summary = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'cases': {}}
for name in cases:
    wire = (EV / name / 'request.jsonl').read_text()
    d = out / name
    d.mkdir(exist_ok=True)
    r = subprocess.run([str(binary), '--font-dir', str(stage / 'fonts'), '--v2', str(d / 'display.json')], input=wire, text=True, capture_output=True, env=env, timeout=60)
    (d / 'stdout.jsonl').write_text(r.stdout)
    (d / 'stderr.txt').write_text(r.stderr)
    disp = json.loads((d / 'display.json').read_text())
    ref = json.loads((EV / name / 'display.json').read_text())
    def runs(dl):
        items = []
        dl = dl.get('payload', dl)
        for page in dl.get('pages', []):
            for it in page.get('items', []):
                if 'glyphs' in it:
                    items.append({'text': it.get('text'), 'font': it.get('font_id') or it.get('font'), 'size': it.get('font_size'),
                                  'glyphs': [{k: g[k] for k in ('gid', 'origin_x', 'baseline_y', 'advance_x', 'cluster')} for g in it['glyphs']]})
        return items
    same = json.dumps(disp, sort_keys=True) == json.dumps(ref, sort_keys=True)
    summary['cases'][name] = {
        'returncode': r.returncode,
        'identical_to_evidence': same,
        'diagnostics': disp.get('payload', disp).get('diagnostics'),
        'runs': runs(disp),
        'evidence_runs': runs(ref),
        'display_sha256': hashlib.sha256((d / 'display.json').read_bytes()).hexdigest(),
    }
(out / 'summary.json').write_text(json.dumps(summary, indent=1) + '\n')
for name, c in summary['cases'].items():
    print(name, 'rc', c['returncode'], 'identical' if c['identical_to_evidence'] else 'DIFFERS', 'diags', [d.get('code') for d in (c['diagnostics'] or [])])
    for run in c['runs']:
        print('   ', repr(run['text']), [(g['gid'], g['origin_x'], g['advance_x']) for g in run['glyphs']])
