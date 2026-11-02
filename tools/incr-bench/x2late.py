#!/usr/bin/env python3
"""x2late.py ENGINE... : letter@x2-late (Infinite Descent x2, page 887) on each engine built by
mkeng.sh in $INCR_BENCH_DIR, one flashtex-host --socket session each, through t7.py's Host and
keys(). Prints, per engine, every compile's edited_instr_k / typeset_instr_k / to_edited_instr_k /
rerun_pages from the host's DONE stages, and the median over steady keys 2-7.

Throwaway measurement driver for #1927 (lane FANCYHDR); not for main."""
import json, os, shutil, statistics, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__))))
import t7, docspec  # noqa: E402

IB = t7.IB
DOC = 'infdesc-x2'
ARGS = ['--edit', 'x2/book/real-numbers/inequalities.tex', '--page', '887', '--kind', 'letter',
        '--keys', '8', '--gap-ms', '300']
WANT = ('edited_instr_k', 'typeset_instr_k', 'to_edited_instr_k', 'restore_instr_k', 'rerun_pages')


def find(o, out):
    if isinstance(o, dict):
        for k, v in o.items():
            if k in WANT and isinstance(v, (int, float)):
                out.setdefault(k, v)
            else:
                find(v, out)
    elif isinstance(o, list):
        for v in o:
            find(v, out)
    return out


def one(name, outdir):
    eng = f'{IB}/{name}'
    work = f'{IB}/x2w-{name}'
    s0 = f'{IB}/x2s0-{name}'
    sock = f'{IB}/x2-{name}.sock'
    shutil.rmtree(work, ignore_errors=True)
    shutil.rmtree(s0, ignore_errors=True)
    os.makedirs(f'{work}/out')
    docspec.copy(IB, DOC, work)
    h = t7.Host(eng, sock, s0, f'{outdir}/{name}.host-stderr', [])
    try:
        recs, err = t7.keys(eng, sock, work, ARGS, f'{outdir}/{name}.jsonl', 3600)
    finally:
        rss = h.stop()
    dones = [find(r['done'], {}) for r in recs if 'done' in r]
    lat = [r['edited_page_ms'] for r in recs if 'key' in r]
    return dict(engine=name, head=open(f'{eng}/HEAD').read().strip(), where=err, dones=dones,
                edited_page_ms=lat, peak_rss_gb=round((rss or 0) / 2**30, 2))


def med(dones, k):
    # dones[0] is the open; keys 2-7 are dones[2:8]
    v = [d[k] for d in dones[2:8] if k in d]
    return round(statistics.median(v) / 1000, 1) if v else None


def main():
    outdir = os.environ.get('X2_OUT', f'{IB}/x2-out')
    os.makedirs(outdir, exist_ok=True)
    res = [one(n, outdir) for n in sys.argv[1:]]
    json.dump(res, open(f'{outdir}/x2late.json', 'w'), indent=1)
    print('| engine | HEAD | edited_instr (M) | typeset_instr (M) | to_edited_instr (M) | rerun_pages | edited_page_ms median | peak RSS GB |')
    print('|---|---|---:|---:|---:|---:|---:|---:|')
    for r in res:
        d = r['dones']
        rp = [x.get('rerun_pages') for x in d[2:8]]
        ms = r['edited_page_ms'][2:8]
        print(f"| {r['engine']} | {r['head']} | {med(d, 'edited_instr_k')} | {med(d, 'typeset_instr_k')} | "
              f"{med(d, 'to_edited_instr_k')} | {rp} | {round(statistics.median(ms), 1) if ms else None} | {r['peak_rss_gb']} |")
    for r in res:
        print(f"{r['engine']} per compile edited_instr_k: {[x.get('edited_instr_k') for x in r['dones']]}")


if __name__ == '__main__':
    main()
