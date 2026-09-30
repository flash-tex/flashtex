#!/bin/bash
# keys.sh ENGINE DOC AT [TAG]: keystroke -> edited page through the socket (dl3-keys, the
# client side of the socket as the app sees it), waiting for each DONE, on
# /tmp/p4f/docs/DOC. The host is started as the app starts it (--s0-cache, --once).
# ENGINE is /tmp/p4f/NAME from mkeng.sh. Output: /tmp/p4f/keys/DOC-AT-TAG.jsonl.
set -e
S=$(cd "$(dirname "$0")" && pwd)
E=$1; DOC=$2; AT=$3; TAG=${4:-$1}
B=/tmp/p4f/keys
KEYS=${KEYS:-40}
export FLASHTEX_POOL=/tmp/p4f/$E/pdftex.pool
export FLASHTEX_FORMATS=/tmp/p4f/fmt-$E
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
W=$B/work-$DOC
rm -rf $W; mkdir -p $W/out
cp /tmp/p4f/docs/$DOC/main.tex $W/main.tex
rm -rf $B/s0-$DOC $B/h.sock
$S/to.sh 600 /tmp/p4f/$E/flashtex-host --socket $B/h.sock --s0-cache $B/s0-$DOC --once $HOSTARGS > $B/h.out 2> $B/h.err &
HP=$!
for i in $(seq 1 600); do grep -q listening $B/h.out 2>/dev/null && break; sleep 0.05; done
$S/to.sh 600 /tmp/p4f/$E/dl3-keys --socket $B/h.sock --root $W --main main.tex --output-dir $W/out --keys $KEYS --at $AT $KEYARGS > $B/$DOC-$AT-$TAG.jsonl || echo "dl3-keys failed"
kill $HP 2>/dev/null; wait $HP 2>/dev/null || true
echo "== $DOC at $AT $TAG: $(grep summary $B/$DOC-$AT-$TAG.jsonl | cut -c1-600)"
