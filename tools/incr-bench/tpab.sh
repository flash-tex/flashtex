#!/usr/bin/env bash
# tpab.sh WORK REF0 REF1 ...: measurement only (lane REVIEW-BOXMEMO, not THROUGHPUT's A/B): the cost
# of #2091 (fonts keyed by what they are) and #2102 (printed names keyed) on edited keystrokes, each
# REF against REF0 (main), with BOX-MEMO on, on the PC: t7.py letter@SITE (12 keys) at long-deck,
# beamer-madrid and infdesc-x2's x2-late page, and full-100 letter@middle; engines interleaved per
# site. Each REF gets the T7 real-document harness of 3fbeb7486 (lane BEAMER-FRAME's, #1889's)
# applied on top, with infdesc-x2's sites from #1889's realdocs.json (e394fef53).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
TBASE=${CARGO_TARGET_DIR_BASE:-$WORK/target}
HARNESS=3fbeb748668259cd1b3e43feea9ebb88dc976ed1
REALDOCS_1889=e394fef53
i=0
for REF in "$@"; do
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$REF" >/dev/null
  echo "e$i: $REF = $(git -C "$src" rev-parse --short HEAD)" | tee -a "$WORK/r/engines.txt"
  git -C "$src" -c user.name=m -c user.email=m@invalid cherry-pick -n "$HARNESS" >/dev/null
  git -C "$ROOT" show "$REALDOCS_1889:tools/incr-bench/realdocs.json" > "$WORK/realdocs-1889.json"
  python3 - "$src/tools/incr-bench/realdocs.json" "$WORK/realdocs-1889.json" <<'PY'
import json, sys
mine, theirs = sys.argv[1], sys.argv[2]
d = json.load(open(mine)); t = json.load(open(theirs))
d['docs']['infdesc-x2'] = t['docs']['infdesc-x2']
json.dump(d, open(mine, 'w'), indent=1)
PY
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo build --release -q -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" bash tools/incr-bench/mkeng.sh "e$i" >/dev/null)
  i=$((i + 1))
done
cd "$WORK/src-0"
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid infdesc-x2 >/dev/null
SITES=${COV_SITES:-long-deck:middle long-deck:start beamer-madrid:middle beamer-madrid:start infdesc-x2:x2-late full-100:middle}
for t in $SITES; do
  doc=${t%%:*}; site=${t#*:}
  for j in $(seq 0 $((i - 1))); do
    e=e$j
    echo "== $doc $site $e"
    if [ "$doc" = full-100 ]; then
      args=(--docs full-100 --phases "letter@$site")
    else
      args=(--docs "$doc" --phases letter@site --sites "$site")
    fi
    FLASHTEX_BOXMEMO=on FLASHTEX_BOXMEMO_STATS="$WORK/r/$doc-$site-$e.stats" \
      python3 tools/incr-bench/t7.py --engine $e "${args[@]}" --keys 12 \
      --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/$doc-$site-$e" >/dev/null 2>&1 || echo "  exit $?"
  done
done
python3 - "$WORK/r" <<'PY' > "$WORK/r/summary.md"
import glob, json, os, sys
r = sys.argv[1]
def boxmemo(d):
    """BOX-MEMO's session counts (offers, hits, committed, entered) at the first and last DONE."""
    seen = []
    for f in sorted(glob.glob(os.path.join(d, 'raw', '*'))):
        try:
            for line in open(f, errors='replace'):
                i = line.find('"boxmemo":[')
                if i >= 0:
                    seen.append(json.loads(line[i + 10:line.index(']', i) + 1]))
        except OSError:
            pass
    return (seen[0], seen[-1]) if seen else (None, None)
print('| run | phase | edited instr M p50 | max | to-edited M p50 | BOX-MEMO first/last DONE [offers, hits, committed, entered] |')
print('|---|---|---:|---:|---:|---|')
for d in sorted(glob.glob(os.path.join(r, '*-e[0-9]'))):
    try:
        s = json.load(open(os.path.join(d, 'summary.json')))
    except (OSError, ValueError):
        print(f'| {os.path.basename(d)} | (no summary) | | | | |'); continue
    rows = []
    def walk(x):
        if isinstance(x, dict):
            if 'edited_instr_m' in x:
                rows.append(x)
            for v in x.values():
                walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(s)
    a, b = boxmemo(d)
    for x in rows:
        e = x.get('edited_instr_m') or {}
        t = x.get('to_edited_instr_m') or {}
        print(f"| {os.path.basename(d)} | {x.get('phase', x.get('name', ''))} | {e.get('p50')} | {e.get('max')} | {t.get('p50')} | {a} / {b} |")
PY
cat "$WORK/r/summary.md"
cd "$WORK"
for j in $(seq 0 $((i - 1))); do git -C "$ROOT" worktree remove --force "$WORK/src-$j" || true; done
