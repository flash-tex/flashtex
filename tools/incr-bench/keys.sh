#!/usr/bin/env bash
# keys.sh ENGINE DOC AT [TAG]: keystroke -> edited page through the socket (dl3-keys, the client
# side of the socket as the app sees it), waiting for each DONE, on $INCR_BENCH_DIR/docs/DOC. The
# host is started as the app starts it (--s0-cache, --once). KEYS (default 40), KEYARGS (dl3-keys:
# --no-viewport --gap-ms MS --page P --where W --sentence --overlap), HOSTARGS (the host's).
# Output: $INCR_BENCH_DIR/keys/DOC-AT-TAG.jsonl; the host's stderr in $INCR_BENCH_DIR/keys/h.err.
set -e
IB=${INCR_BENCH_DIR:-/tmp/incr-bench}
S=$(cd "$(dirname "$0")" && pwd)
E=$1; DOC=$2; AT=$3; TAG=${4:-$1}
B=$IB/keys
KEYS=${KEYS:-40}
export FLASHTEX_POOL=$IB/$E/pdftex.pool
export FLASHTEX_FORMATS=$IB/fmt-$E
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
W=$B/work-$DOC
rm -rf $W; mkdir -p $W/out
cp $IB/docs/$DOC/main.tex $W/main.tex
rm -rf $B/s0-$DOC $B/h.sock
$S/to.sh 600 $IB/$E/flashtex-host --socket $B/h.sock --s0-cache $B/s0-$DOC --once $HOSTARGS > $B/h.out 2> $B/h.err &
HP=$!
for i in $(seq 1 600); do grep -q listening $B/h.out 2>/dev/null && break; sleep 0.05; done
$S/to.sh 600 $IB/$E/dl3-keys --socket $B/h.sock --root $W --main main.tex --output-dir $W/out --keys $KEYS --at $AT $KEYARGS > $B/$DOC-$AT-$TAG.jsonl || echo "dl3-keys failed"
kill $HP 2>/dev/null; wait $HP 2>/dev/null || true
echo "== $DOC at $AT $TAG: $(grep summary $B/$DOC-$AT-$TAG.jsonl | cut -c1-600)"
