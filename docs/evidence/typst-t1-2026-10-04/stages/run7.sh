#!/bin/bash
# Stage counters: where the first page's time goes past the compile.
F=/Users/jay3332/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/typst-assets-0.15.1/files/fonts
OUT=${OUT:-/tmp/ftl/stages.jsonl}
load1() { sysctl -n vm.loadavg | awk '{print $2}'; }
run() { # bindir label doc keys [host args...]
  local bd=$1 label=$2 doc=$3 keys=$4; shift 4
  local ha=()
  for a in "$@"; do ha+=(--host-arg "$a"); done
  local waited=0
  while awk -v l="$(load1)" 'BEGIN{exit !(l>=20)}'; do sleep 10; waited=$((waited+10)); if [ $waited -ge 1800 ]; then echo "{\"label\":\"$label\",\"aborted\":\"load never fell below 20 in 30 min\"}" >> $OUT; return; fi; done
  $bd/typing_bench run --host $bd/flashtex-typst-host --fonts $F --doc /tmp/ftl/docs/$doc --keys $keys --label $label "${ha[@]}" >> $OUT 2>>$OUT.err &
  local pid=$!
  local high=0
  echo "{\"label\":\"$label\",\"start_load\":$(load1)}" >> $OUT.load
  while kill -0 $pid 2>/dev/null; do
    sleep 5
    local l=$(load1)
    echo "{\"label\":\"$label\",\"load1\":$l}" >> $OUT.load
    if awk -v l="$l" 'BEGIN{exit !(l>25)}'; then high=$((high+1)); else high=0; fi
    if [ $high -ge 3 ]; then
      pkill -P $pid; kill $pid; wait $pid 2>/dev/null
      echo "{\"label\":\"$label\",\"aborted\":\"load above 25 for 15 s ($l)\"}" >> $OUT
      return
    fi
  done
  wait $pid
}
BD=/tmp/ftl/bin4
run $BD p300-noimg p300-noimg 20 --verify off --rss-ceiling-mb 16384
run $BD p1000-noimg p1000-noimg 12 --verify off --rss-ceiling-mb 16384
run $BD p1000-spanix p1000 12 --verify off --rss-ceiling-mb 16384
echo all-done
