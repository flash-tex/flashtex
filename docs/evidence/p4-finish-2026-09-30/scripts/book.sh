#!/bin/bash
# book.sh ENGINE TAG PAGE WHERE [--sentence]: the owner's 1,072-page test document
# (~/Documents/FlashTeX-1000-page-test/book.tex, copied; the owner's files and app are not
# touched): one host session of our own (private S0 cache under /tmp/p4f), KEYS keystrokes
# GAP ms apart on the prose line nearest to PAGE (0-based), at WHERE (start|middle|end of the
# line), a letter or with --sentence twelve words, each followed by its revert.
# Output: /tmp/p4f/book/TAG-PAGE-WHERE[-sentence].jsonl, the host's stderr beside it.
set -e
S=$(cd "$(dirname "$0")" && pwd)
E=$1; TAG=$2; PAGE=$3; WHERE=$4; SENT=$5
B=/tmp/p4f/book
N=$TAG-$PAGE-$WHERE${SENT:+-sentence}
export FLASHTEX_POOL=/tmp/p4f/$E/pdftex.pool
export FLASHTEX_FORMATS=/tmp/p4f/fmt-$E
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
W=$B/work-$N
rm -rf $W $B/s0-$N $B/$N.sock; mkdir -p $W/out
cp ~/Documents/FlashTeX-1000-page-test/book.tex $W/book.tex
$S/to.sh 1800 /tmp/p4f/$E/flashtex-host --socket $B/$N.sock --s0-cache $B/s0-$N --once $HOSTARGS > $B/$N.out 2> $B/$N.err &
HP=$!
for i in $(seq 1 600); do grep -q listening $B/$N.out 2>/dev/null && break; sleep 0.05; done
$S/to.sh 1800 /tmp/p4f/$E/dl3-keys --socket $B/$N.sock --root $W --main book.tex --output-dir $W/out \
  --keys ${KEYS:-6} --gap-ms ${GAP:-300} --no-viewport --page $PAGE --where $WHERE $SENT > $B/$N.jsonl 2>> $B/$N.err || echo "dl3-keys failed"
kill $HP 2>/dev/null; wait $HP 2>/dev/null || true
rm -rf $W $B/s0-$N
grep "typing on" $B/$N.err | tail -1
