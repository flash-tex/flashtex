#!/usr/bin/env bash
# sites.sh TAG DOCDIR MAIN KEYS "FILE:LINE:PAGE" ...: one resident host on a copy of DOCDIR; for each
# site, dl3-keys types KEYS letter edits at FILE's LINE watching 0-based PAGE. With PROF=1 the macro
# profile of every edit window goes to $OUT/prof.N (FLASHTEX_MACRO_PROFILE in the host).
# ENG (default prof) picks the engine under $IB.
set -e
IB=${IB:-$HOME/ib-infpage}; ENG=${ENG:-prof}; E=$IB/$ENG
TAG=$1; SRC=$2; MAIN=$3; KEYS=$4; shift 4
OUT=$IB/out/$TAG; W=$IB/work-$TAG
rm -rf $OUT; mkdir -p $OUT
if [ ! -d $W ]; then cp -r $SRC $W; mkdir -p $W/out; fi
export FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$IB/fmt-$ENG SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
if [ -n "$PROF" ]; then export FLASHTEX_MACRO_PROFILE=$OUT/prof; fi
export FLASHTEX_PERF_MARKS=$OUT/marks
S=$OUT/h.sock
$E/flashtex-host --socket $S --s0-cache $W/s0-$ENG $HOSTARGS > $OUT/h.out 2> $OUT/h.err &
HP=$!
for i in $(seq 1 600); do grep -q listening $OUT/h.out 2>/dev/null && break; sleep 0.05; done
n=0
for site in "$@"; do
  IFS=: read F L P <<< "$site"
  if [ "$P" = "?" ]; then
    P=$($E/dl3-keys --socket $S --root $W --main $MAIN --output-dir $W/out --edit $F --line $L --page 0 --keys 1 2>&1 >/dev/null | sed -n 's/.*is on pages \[\([0-9]*\)\].*/\1/p')
    if [ -z "$P" ]; then P=0; fi
  fi
  $E/dl3-keys --socket $S --root $W --main $MAIN --output-dir $W/out --edit $F --line $L --page $P --keys $KEYS --gap-ms 300 $KEYARGS > $OUT/site$n.jsonl 2> $OUT/site$n.err || echo "dl3-keys failed on $site: $(tail -2 $OUT/site$n.err)"
  echo "site$n $F:$L:$P"
  n=$((n+1))
done
kill $HP 2>/dev/null; wait $HP 2>/dev/null || true
