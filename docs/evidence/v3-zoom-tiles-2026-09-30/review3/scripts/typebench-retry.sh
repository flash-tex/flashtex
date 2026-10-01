#!/bin/bash
# usage: DOCFROM=... AT=... a38-typebench-retry.sh <label> <ppp> <ms>: a38-typebench.sh until the pane is at least 600x600 pt (5 attempts).
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
for attempt in 1 2 3 4 5; do
  bash $S/a38-typebench.sh "$@"
  J=$S/a38-typebench/$1/typing.json
  if [ -f "$J" ] && python3 -c "
import json,re,sys
d=json.load(open('$J')); v=[float(x) for x in re.findall(r'[\d.]+', d.get('pane',''))]
sys.exit(0 if len(v)==4 and v[2]>=600 and v[3]>=600 else 1)"; then
    echo "$1 ok after $attempt" >> $S/logs/a38-typeretries.log; exit 0
  fi
  echo "$1 attempt $attempt discarded: pane $(python3 -c "import json;print(json.load(open('$J')).get('pane'))" 2>/dev/null || echo no-result)" >> $S/logs/a38-typeretries.log
done
