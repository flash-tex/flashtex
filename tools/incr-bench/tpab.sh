#!/usr/bin/env bash
# tpab.sh WORK REF0 REF1 ...: measurement only (lane REVIEW-BOXMEMO, not THROUGHPUT's A/B), on the PC:
#  A. cold opens (t7.py --phases open: dl3-coldopen, every DONE's instructions) of infdesc-x2,
#     full-100 and long-deck, engines e0..e2 interleaved, two rounds;
#  B. cross-document hits: one host (the last engine) serving the T7 documents one after another,
#     twice round, 4 letters each, BOX-MEMO's counts (stages.boxmemo: offers, hits, committed,
#     entered, xdoc_hits) after each document.
# A REF may be BASE+COMMIT: COMMIT cherry-picked onto BASE. Each engine gets the T7 real-document
# harness of 3fbeb7486 (lane BEAMER-FRAME's, #1889's) on top, with infdesc-x2's sites from #1889's
# realdocs.json (e394fef53).
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
git -C "$ROOT" show "$REALDOCS_1889:tools/incr-bench/realdocs.json" > "$WORK/realdocs-1889.json"
i=0
for SPEC in "$@"; do
  REF=${SPEC%%+*}; PICK=${SPEC#"$REF"}; PICK=${PICK#+}
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$REF" >/dev/null
  if [ -n "$PICK" ]; then git -C "$src" -c user.name=m -c user.email=m@invalid cherry-pick "$PICK" >/dev/null; fi
  echo "e$i: $SPEC = $(git -C "$src" rev-parse --short HEAD)" | tee -a "$WORK/r/engines.txt"
  git -C "$src" -c user.name=m -c user.email=m@invalid cherry-pick -n "$HARNESS" >/dev/null
  python3 - "$src/tools/incr-bench/realdocs.json" "$WORK/realdocs-1889.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); t = json.load(open(sys.argv[2]))
d['docs']['infdesc-x2'] = t['docs']['infdesc-x2']
json.dump(d, open(sys.argv[1], 'w'), indent=1)
PY
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo build --release -q -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" bash tools/incr-bench/mkeng.sh "e$i" >/dev/null)
  i=$((i + 1))
done
last=$((i - 1))
cd "$WORK/src-0"
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid infdesc-x2 >/dev/null
python3 tools/incr-bench/mkdocs.py >/dev/null

# A. cold opens: e0..e2 (the counter engine e3 is not timed)
nA=$(( last < 2 ? last : 2 ))
for round in 1 2; do
  for doc in ${OPEN_DOCS:-full-100 long-deck infdesc-x2}; do
    for j in $(seq 0 $nA); do
      echo "== open $doc e$j round $round"
      FLASHTEX_BOXMEMO=on python3 tools/incr-bench/t7.py --engine e$j --docs "$doc" --phases open \
        --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/open-$doc-e$j-$round" >/dev/null 2>&1 || echo "  exit $?"
    done
  done
done

# B. cross-document hits: one host, the documents in turn
E="$INCR_BENCH_DIR/e$last"
X="$WORK/xdoc"; rm -rf "$X"; mkdir -p "$X/s0"
SOCK="$X/host.sock"
FLASHTEX_BOXMEMO=on FLASHTEX_POOL="$E/pdftex.pool" FLASHTEX_FORMATS="$INCR_BENCH_DIR/fmt-e$last" \
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 \
  "$E/flashtex-host" --socket "$SOCK" --s0-cache "$X/s0" < /dev/null > "$X/host.out" 2> "$X/host.err" &
HOST=$!
for _ in $(seq 1 600); do grep -q listening "$X/host.out" 2>/dev/null && break; sleep 0.5; done
for round in 1 2; do
  for doc in ${XDOC_DOCS:-plain-10 full-10 plain-100 full-100 beamer-5 beamer-30 long-deck beamer-madrid}; do
    w="$X/$doc"; rm -rf "$w"; mkdir -p "$w/out"; cp -R "$INCR_BENCH_DIR/docs/$doc/." "$w/"
    "$E/dl3-keys" --socket "$SOCK" --root "$w" --main main.tex --output-dir "$w/out" --keys 4 --at 0.5 --kind letter \
      > "$X/$doc-$round.jsonl" 2> "$X/$doc-$round.err" || echo "  $doc exit $?"
    c=$(grep -o '"boxmemo":\[[0-9,]*\]' "$X/$doc-$round.jsonl" | tail -1)
    echo "xdoc round $round $doc $c" | tee -a "$WORK/r/xdoc.txt"
  done
done
kill "$HOST" 2>/dev/null || true
wait "$HOST" 2>/dev/null || true
cp "$X"/*.jsonl "$X/host.err" "$WORK/r/" 2>/dev/null || true

python3 - "$WORK/r" <<'PY' > "$WORK/r/summary.md"
import glob, json, os, sys
r = sys.argv[1]
print('| open | DONEs | instructions (sum of every DONE) |')
print('|---|---:|---:|')
for d in sorted(glob.glob(os.path.join(r, 'open-*'))):
    tot, n = 0, 0
    for f in glob.glob(os.path.join(d, 'raw', '*-open.jsonl')):
        for line in open(f, errors='replace'):
            if not line.startswith('{'):
                continue
            try:
                j = json.loads(line)
            except ValueError:
                continue
            for x in j.get('dones', []):
                v = x[-1] if isinstance(x, list) else None
                if isinstance(v, (int, float)):
                    tot += v; n += 1
    print(f'| {os.path.basename(d)} | {n} | {tot} |')
print()
print('```'); print(open(os.path.join(r, 'xdoc.txt')).read() if os.path.exists(os.path.join(r, 'xdoc.txt')) else ''); print('```')
PY
cat "$WORK/r/summary.md"
cd "$WORK"
for j in $(seq 0 $last); do git -C "$ROOT" worktree remove --force "$WORK/src-$j" || true; done
