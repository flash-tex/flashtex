#!/bin/bash
# Idle-machine latency run. Each run waits for the 1-minute load to be < 20,
# then runs with a watcher sampling the load every 5 s; 3 samples in a row
# above 25 abort that run (the bench is killed by PID).
B=/tmp/ftl/bin/typing_bench
H=/tmp/ftl/bin/flashtex-typst-host
F=/Users/jay3332/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/typst-assets-0.15.1/files/fonts
OUT=/tmp/ftl/latency.jsonl
load1() { sysctl -n vm.loadavg | awk '{print $2}'; }
run() { # label doc keys [host args...]
  local label=$1 doc=$2 keys=$3; shift 3
  local ha=()
  for a in "$@"; do ha+=(--host-arg "$a"); done
  local waited=0
  while awk -v l="$(load1)" 'BEGIN{exit !(l>=20)}'; do sleep 10; waited=$((waited+10)); if [ $waited -ge 1800 ]; then echo "{\"label\":\"$label\",\"aborted\":\"load never fell below 20 in 30 min\"}" >> $OUT; return; fi; done
  $B run --host $H --fonts $F --doc /tmp/ftl/docs/$doc --keys $keys --label $label "${ha[@]}" >> $OUT 2>>$OUT.err &
  local pid=$!
  local high=0
  echo "{\"label\":\"$label\",\"start_load\":$(load1)}" >> /tmp/ftl/load.jsonl
  while kill -0 $pid 2>/dev/null; do
    sleep 5
    local l=$(load1)
    echo "{\"label\":\"$label\",\"load1\":$l}" >> /tmp/ftl/load.jsonl
    if awk -v l="$l" 'BEGIN{exit !(l>25)}'; then high=$((high+1)); else high=0; fi
    if [ $high -ge 3 ]; then
      kill $pid; wait $pid 2>/dev/null
      echo "{\"label\":\"$label\",\"aborted\":\"load above 25 for 15 s ($l)\"}" >> $OUT
      return
    fi
  done
  wait $pid
}
run p100-seeded p100 40 --verify off
run p100-standard p100 20 --seeded off --verify off
run p300-seeded p300 40 --verify off
run p300-standard p300 20 --seeded off --verify off
run p1000-seeded p1000 20 --verify off
run p1000-standard p1000 6 --seeded off --verify off
echo all-done
