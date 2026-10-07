#!/usr/bin/env python3
"""coldopen.py: a document's first open through the socket host, as the app opens it (lane COLD-OPEN,
DESIGN.md §1.2 "Opening": the visible page <= 1 s after open, the rest streaming; edits during the
initial compile as fast as after it).

    coldopen.py ENGINE DOCDIR MAIN OUT.jsonl [--reps N] [--viewport P] [--edit-at-ms MS --edit-line N
        [--edit-file F] [--edit-viewport]] [--keep-aux] [--host-arg ARG]... [--tag TAG]

Each rep: a fresh copy of DOCDIR (its output directory empty unless --keep-aux, which keeps the
previous rep's output as a reopen from the project's last copy would), a fresh host
(`flashtex-host --socket`, a fresh S0 cache, warmed as the app's is: `listening` comes after the
warm-up), then `dl3-coldopen` (one COMPILE, optionally a keystroke during it). One JSON line per rep
in OUT.jsonl: dl3-coldopen's line plus the host's start-up time (`ready_ms`), its peak RSS, load1 and
the tag; the host's stderr is appended to OUT.jsonl.host-stderr. The engine thread's instruction counts are in `host.stages` (Linux: perf_event_open; macOS:
the kernel's per-thread counters): compare engines by those on a loaded machine; wall times are
references only on a quiet one.

ENGINE is an incr-bench engine directory (`mkeng.sh NAME`: `$INCR_BENCH_DIR/NAME`, with
`dl3-coldopen` beside `dl3-keys`) and its format in `$INCR_BENCH_DIR/fmt-NAME`.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from t7 import Host, load1  # noqa: E402

S = os.path.dirname(os.path.abspath(__file__))
# A previous run's files in DOCDIR are not copied: the open starts with none of them.
OUTPUTS = ('out', '*.aux', '*.out', '*.toc', '*.lof', '*.lot', '*.log', '*.pdf', '*.idx', '*.ind', '*.ilg', '*.hnt',
           '*.sol', '*.thm', '*.bbl', '*.blg', '*.bcf', '*.run.xml', '*.synctex.gz', '*.fls', '*.fdb_latexmk')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('engine')
    ap.add_argument('docdir')
    ap.add_argument('main')
    ap.add_argument('out')
    ap.add_argument('--reps', type=int, default=1)
    ap.add_argument('--viewport', type=int)
    ap.add_argument('--edit-at-ms', type=int)
    ap.add_argument('--edit-line', type=int)
    ap.add_argument('--edit-file')
    ap.add_argument('--edit-viewport', action='store_true')
    ap.add_argument('--keep-aux', action='store_true')
    ap.add_argument('--external-tools')
    ap.add_argument('--host-arg', action='append', default=[])
    ap.add_argument('--tag', default='')
    ap.add_argument('--timeout', type=int, default=900)
    a = ap.parse_args()
    eng = os.path.abspath(a.engine)
    tmp = tempfile.mkdtemp(prefix='coldopen-')
    work = os.path.join(tmp, 'doc')
    keep = None
    try:
        for rep in range(a.reps):
            if os.path.exists(work):
                shutil.rmtree(work)
            shutil.copytree(a.docdir, work, ignore=shutil.ignore_patterns(*OUTPUTS))
            os.makedirs(f'{work}/out', exist_ok=True)
            if a.keep_aux and keep:
                shutil.copytree(keep, f'{work}/out', dirs_exist_ok=True)
            s0 = os.path.join(tmp, f's0-{rep}')
            os.makedirs(s0)
            sock = os.path.join(tmp, 'host.sock')
            h = Host(eng, sock, s0, a.out + '.host-stderr', a.host_arg)
            cmd = [f'{S}/to.sh', str(a.timeout), f'{eng}/dl3-coldopen', '--socket', sock, '--root', work,
                   '--main', a.main, '--output-dir', f'{work}/out']
            if a.viewport is not None:
                cmd += ['--viewport', str(a.viewport)]
            if a.edit_at_ms is not None:
                cmd += ['--edit-at-ms', str(a.edit_at_ms), '--edit-line', str(a.edit_line)]
                if a.edit_file:
                    cmd += ['--edit-file', a.edit_file]
                if a.edit_viewport:
                    cmd += ['--edit-viewport']
            if a.external_tools:
                cmd += ['--external-tools', a.external_tools]
            l0 = load1()
            p = subprocess.run(cmd, capture_output=True, text=True)
            rss = h.stop()
            lines = [l for l in p.stdout.splitlines() if l.startswith('{')]
            if p.returncode != 0 or not lines:
                print(f'rep {rep}: dl3-coldopen exit {p.returncode}: {p.stderr.strip()[-600:]}', file=sys.stderr)
                sys.exit(1)
            r = json.loads(lines[-1])
            r.update(rep=rep, tag=a.tag, ready_ms=round(h.ready_ms, 1), peak_rss=rss, load1=[l0, load1()],
                     keep_aux=bool(a.keep_aux and keep))
            # the open's compiles (its tool follow-ups too): passes, their modes, instructions
            mine = [d for d in r['dones'] if d[0] == 1]
            r['passes_all'] = sum(d[3] or 0 for d in mine)
            r['pass_modes_all'] = [m for d in mine for m in (d[4] or [])]
            r['instr_g_all'] = round(sum(d[5] or 0 for d in mine) / 1e6, 3)
            with open(a.out, 'a') as f:
                f.write(json.dumps(r) + '\n')
            print(f"rep {rep}: pages {r['pages']} first {r['first_page_ms'] or 0:.0f} all {r['all_seen_ms'] or 0:.0f} "
                  f"done {r['done_ms']:.0f} ms, passes {r['passes_all']} {r['pass_modes_all']}, "
                  f"{r['instr_g_all']} G instr"
                  + (f", edit page {r.get('edited_page')} in {r.get('edited_page_ms')} ms (open at page "
                     f"{r.get('open_pages_then')}, pass {r.get('open_pass_then')})" if a.edit_at_ms is not None else ''),
                  flush=True)
            keep = os.path.join(tmp, 'kept-out')
            if os.path.exists(keep):
                shutil.rmtree(keep)
            shutil.copytree(f'{work}/out', keep)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == '__main__':
    main()
