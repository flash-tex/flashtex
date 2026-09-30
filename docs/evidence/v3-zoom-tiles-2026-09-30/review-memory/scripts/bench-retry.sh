#!/bin/bash
# usage: DOCFROM=... a38-bench-retry.sh <label> <ppp>: a38-bench.sh until the recorded pane is >= 600x600 pt (5 attempts).
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
for attempt in 1 2 3 4 5; do
  bash $S/a38-bench.sh "$1" "$2"
  J=$S/a38-bench/$1/scroll.json
  if [ -f "$J" ] && python3 -c "
import json,re,sys
d=json.load(open('$J')); w,h=[float(x) for x in re.findall(r'[\d.]+', d.get('pane','0'))[2:4]]
sys.exit(0 if w>=600 and h>=600 else 1)"; then
    echo "$1 ok after $attempt" >> $S/logs/a38-retries.log; exit 0
  fi
  echo "$1 attempt $attempt discarded: $(grep -o '"pane" : "[^"]*"' $J 2>/dev/null || echo no-result)" >> $S/logs/a38-retries.log
done
