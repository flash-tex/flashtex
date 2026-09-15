#!/usr/bin/env bash
# usage: sampler.sh <typing-bench build dir> <cellname> <outfile>
# waits for the cell's log to say "bench: typing", then samples FlashTeXMac for 5 s at 1 ms
dir="$1"; cell="$2"; out="$3"
for _ in $(seq 1 3600); do
  log="$(ls -t "$dir"/*/"$cell".log 2>/dev/null | head -1)"
  if [[ -n "$log" ]] && grep -q "bench: typing" "$log" 2>/dev/null; then
    pid="$(pgrep -x FlashTeXMac | head -1)"
    [[ -n "$pid" ]] && sample "$pid" 5 1 -mayDie -file "$out" >/dev/null 2>&1
    echo "sampled $pid -> $out"; exit 0
  fi
  sleep 0.5
done
echo "sampler: timed out waiting for $cell"
