#!/bin/bash
# asmcalls.sh BINARY ROUTINE...: for each generated routine (e.g. get_next),
# disassemble it into $OUT/ROUTINE.s and print its instruction count, its
# stack frame (the prologue's `sub sp`) and the calls it makes, so that what
# was inlined into a hot routine, and how heavy its prologue is, can be seen.
# ("frame" is the first `sub sp` in the routine, which may not be its prologue.)
set -e
B=$1; shift
OUT=${OUT:-$HOME/flashtex-wt/d2/prof}
mkdir -p "$OUT"
for r in "$@"; do
  sym=$(nm "$B" | awk -v r="Globals${#r}${r}\$" '$3 ~ r {print $3; exit}')
  [ -n "$sym" ] || { echo "== $r: not found"; continue; }
  objdump --no-show-raw-insn --disassemble-symbols="$sym" "$B" > "$OUT/$r.s"
  n=$(grep -cE '^ *[0-9a-f]+:' "$OUT/$r.s")
  fr=$(grep -m1 -oE 'sub	sp, sp, #0x[0-9a-f]+' "$OUT/$r.s" || echo none)
  echo "== $r: $n instructions, frame $fr, register pairs saved in the prologue $(grep -E '^ *[0-9a-f]+:' "$OUT/$r.s" | head -12 | grep -cE '[[:space:]]stp[[:space:]]+(x|d)[0-9]+, (x|d)[0-9]+, \[sp')"
  grep -oE 'bl	0x[0-9a-f]+ <[^>]+>' "$OUT/$r.s" | sed -E 's/.*<//; s/>//; s/^__RN.*Globals[0-9]+//; s/^__RN.*engine[0-9]+//' \
    | sort | uniq -c | sort -rn | head -14 | awk '{printf "%s×%s ", $2, $1} END {print ""}'
done
