#!/usr/bin/env bash
# keys_at.sh ENGINE TAG FILE PAGE WHERE [--sentence]: keystrokes into a copy of the LaTeX source
# FILE (a single-file document, e.g. a user's 1,000-page book; the original is not touched): one
# host session of our own (private S0 cache), KEYS keystrokes GAP ms apart on the prose line nearest
# to PAGE (0-based), at WHERE (start|middle|end of the line), a letter or with --sentence twelve
# words, each followed by its revert. Output: $INCR_BENCH_DIR/at/TAG-PAGE-WHERE[-sentence].jsonl,
# dl3-keys's stderr beside it (.err), the host's (.host-stderr).
set -e
IB=${INCR_BENCH_DIR:-/tmp/incr-bench}
S=$(cd "$(dirname "$0")" && pwd)
E=$1; TAG=$2; FILE=$3; PAGE=$4; WHERE=$5; SENT=$6
B=$IB/at
N=$TAG-$PAGE-$WHERE${SENT:+-sentence}
MAIN=$(basename "$FILE")
export FLASHTEX_POOL=$IB/$E/pdftex.pool
export FLASHTEX_FORMATS=$IB/fmt-$E
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
W=$B/work-$N
rm -rf $W $B/s0-$N $B/$N.sock $B/$N.err; mkdir -p $W/out
cp "$FILE" $W/$MAIN
$S/to.sh 1800 $IB/$E/flashtex-host --socket $B/$N.sock --s0-cache $B/s0-$N --once $HOSTARGS > $B/$N.out 2> $B/$N.host-stderr &
HP=$!
for i in $(seq 1 600); do grep -q listening $B/$N.out 2>/dev/null && break; sleep 0.05; done
$S/to.sh 1800 $IB/$E/dl3-keys --socket $B/$N.sock --root $W --main $MAIN --output-dir $W/out \
  --keys ${KEYS:-6} --gap-ms ${GAP:-300} --no-viewport --page $PAGE --where $WHERE $SENT > $B/$N.jsonl 2>> $B/$N.err || echo "dl3-keys failed"
kill $HP 2>/dev/null; wait $HP 2>/dev/null || true
rm -rf $W $B/s0-$N
grep "typing on" $B/$N.err | tail -1
