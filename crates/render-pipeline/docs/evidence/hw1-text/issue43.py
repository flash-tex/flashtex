"""Issue #43 route check: one worker session, oversized \\text requests then a valid one."""
import json, os, pathlib, subprocess, sys, hashlib
S = pathlib.Path(__file__).parent
stage = S / 'stage'
binary = S / 'after' / 'target/release/flashtex-render'
env = dict(os.environ, FLASHTEX_TFM_DIRS=str(stage / 'metrics'), FLASHTEX_FONT_DIRS=str(stage / 'fonts'))
def req(i, math):
    src = '\\documentclass[12pt]{article}\n\\begin{document}\n$' + math + '$\n\\end{document}\n'
    return json.dumps({'protocol_version': 1, 'id': i, 'type': 'compile', 'payload': {'project_id': 'p', 'revision': int(i[1:]), 'entry_path': 'main.tex', 'documents': [{'path': 'main.tex', 'text': src}]}})
words = ['b' + 'a' * 3999] + ['a' * 4000] * 15 + ['a' * 1519 + 'z', 'c' + 'a' * 100 + 'd']
big = ' '.join(words)                                      # 65639 entries, two slots
many = '+'.join(['\\text{x}'] * (131072 + 1))              # one atom beyond the handle space
lines = [req('r1', '\\text{' + big + '}'), req('r2', many), req('r3', '\\text{a b}+\\text{ffi}'), req('r4', '\\text{' + big + '}_{i}')]
r = subprocess.run([str(binary), '--font-dir', str(stage / 'fonts')], input='\n'.join(lines) + '\n', text=True, capture_output=True, env=env, timeout=600)
out = {}
for line in r.stdout.splitlines():
    d = json.loads(line)
    if d.get('type') != 'compile_result':
        continue
    p = d['payload']
    items = [it for pg in p.get('pages', []) for it in pg.get('items', [])]
    texts = [it.get('text') for it in items if 'text' in it]
    out[d['id']] = {'status': p.get('status'), 'diagnostics': [(x.get('code'), x.get('message')[:90]) for x in p.get('diagnostics', [])],
                    'items': len(items), 'first_text': texts[:1], 'last_text': texts[-1:] if texts else None,
                    'reply_bytes': len(line)}
print('returncode', r.returncode, 'stderr', r.stderr[-300:])
for k, v in out.items():
    print(k, json.dumps(v))
(S / 'out' / 'issue43.json').write_text(json.dumps({'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'returncode': r.returncode, 'results': out}, indent=1) + '\n')
