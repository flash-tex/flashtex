#!/bin/bash
# profcmp.sh DOC N ENGINE...: sample N compiles of DOC with each engine (from
# a copy of the baseline's settled directory), summarise them with the L6
# lane's sampletop.py into $ROOT/prof/ENGINE-DOC.top, and print each
# engine's total sample count with the self share of $FUNCS (a regex,
# default: the top functions of the baseline profile).
set -e
ROOT=${ROOT:-$HOME/flashtex-wt/d2}
S=$(cd "$(dirname "$0")" && pwd)
L6=$S/../../l6-optimizations-2026-09-29/scripts
DOC=$1; N=$2; shift 2
FUNCS=${FUNCS:-'^(get_next|get_next_file|macro_call|end_token_list|write|divide_scaled|longest_match|get_avail) '}
for e in "$@"; do
  if [ "$e" != base ] && [ "$e" != pdflatex ]; then
    rm -rf "$ROOT/time/$e-$DOC"; cp -R "$ROOT/time/base-$DOC" "$ROOT/time/$e-$DOC"
  fi
  rm -f "$ROOT"/prof/"$e-$DOC"-*.txt
  bash "$S/prof.sh" "$e" "$DOC" "$N" >/dev/null
  python3 "$L6/sampletop.py" "$ROOT"/prof/"$e-$DOC"-*.txt --top 400 > "$ROOT/prof/$e-$DOC.top"
  echo "== $e $DOC: $(head -1 "$ROOT/prof/$e-$DOC.top")"
  sed -n '/top self/,$p' "$ROOT/prof/$e-$DOC.top" | grep -E "$FUNCS" | awk '{printf "  %-16s self %5s %5s%%\n", $1, $4, $5}'
done
