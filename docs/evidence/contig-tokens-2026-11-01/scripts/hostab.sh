#!/usr/bin/env bash
# hostab.sh DOC PAGE GAP ROUNDS ENGINE... (EDIT=FILE: the file typed in, e.g. a book's chapter): lane CONTIG-TOKENS follow-up (review of #2008, item 3).
# The socket host as the app starts it (--s0-cache, --once; checkpoints and BOX-MEMO on, as on
# main), KEYS keystrokes in the middle of a prose line on 0-based page PAGE, GAP ms apart, each
# waiting for DONE (tools/incr-bench/keys.sh's session), engines in a seeded random order per
# round. The whole host process runs under `perf stat` (instructions:u, cycles:u: the cold
# compile, every checkpoint and every keystroke). Output: $OUT/DOC-pPAGE-ENGINE-rN.{jsonl,perf,err}.
set -e
IB=${INCR_BENCH_DIR:?}; OUT=${OUT:?}; PERF=${PERF:?}; S=${S:?}
DOC=$1; PAGE=$2; G=$3; R=$4; shift 4
KEYS=${KEYS:-30}
mkdir -p $OUT
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
for r in $(seq ${START:-1} $R); do
  order=$(printf '%s\n' "$@" | python3 -c "import random,sys; l=sys.stdin.read().split(); random.Random('$DOC-$PAGE-$r').shuffle(l); print(' '.join(l))")
  for E in $order; do
    export FLASHTEX_POOL=$IB/$E/pdftex.pool FLASHTEX_FORMATS=$IB/fmt-$E
    W=$OUT/work; rm -rf $W $OUT/s0 $OUT/h.sock; mkdir -p $W/out
    cp -r $IB/docs/$DOC/. $W/; rm -rf $W/out; mkdir -p $W/out
    tag=$DOC-p$PAGE-$E-r$r
    $S/to.sh ${TO:-900} $PERF stat -x, -o $OUT/$tag.perf -e instructions:u,cycles:u -- \
      $IB/$E/flashtex-host --socket $OUT/h.sock --s0-cache $OUT/s0 --once > $OUT/h.out 2> $OUT/$tag.err &
    HP=$!
    for i in $(seq 1 600); do grep -q listening $OUT/h.out 2>/dev/null && break; sleep 0.05; done
    $S/to.sh ${TO:-900} $IB/$E/dl3-keys --socket $OUT/h.sock --root $W --main main.tex --output-dir $W/out \
      --keys $KEYS --at 0 --no-viewport --gap-ms $G --page $PAGE ${EDIT:+--edit $EDIT} > $OUT/$tag.jsonl || echo "dl3-keys failed: $tag"
    for i in $(seq 1 600); do kill -0 $HP 2>/dev/null || break; sleep 0.1; done
    kill $HP 2>/dev/null || true; wait $HP 2>/dev/null || true
    echo "$tag load=$(cut -d' ' -f1 /proc/loadavg) $(grep -E 'instructions|cycles' $OUT/$tag.perf | cut -d, -f1 | paste -sd' ') $(python3 $S/stages.py $OUT/$tag.jsonl | tail -1 | tr ' ' '\n' | grep -A1 -E '^(client_edited_page|edited_cpu|cpu|test|restore)$' | grep -v -- -- | paste -sd' ' -)"
  done
done
