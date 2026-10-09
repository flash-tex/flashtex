#!/bin/bash
# --verify of the chapter focus on a flat \include book (gen_flat.py): the focused job
# (incr_bench.py --first-line, the host's first line) and the whole document, edits in ch03,
# every compile compared with from-scratch runs. BEFORE=nobefore drops the contents page's
# \pdffilesize{ch03.tex} (the chapter's own \pdffilesize stays).
# INCR_BENCH_DIR/ENG: an engine staged by tools/incr-bench/mkeng.sh; WORK: scratch.
set -u
D=$(cd "$(dirname "$0")" && pwd); W=$(cd "$D/../../../.." && pwd)
S=${WORK:-/tmp/focus-bench}; mkdir -p "$S"
ENG=${ENG:-focus}; TRIALS=${TRIALS:-6}; KINDS=${KINDS:-replace,insert,delete,sentence}
rm -rf "$S/flat"; python3 "$D/gen_flat.py" "$S/flat" 6 ch03 ${BEFORE:-}
python3 "$W/tools/incr-bench/incr_bench.py" "$ENG" "$S/flat" main --edit ch03.tex --trials "$TRIALS" --verify \
  --kinds "$KINDS" --first-line '\AtBeginDocument{\includeonly{ch03}}\input main.tex' \
  --out "$S/verify-focus-$ENG.jsonl" --quiet | tail -1
python3 "$W/tools/incr-bench/incr_bench.py" "$ENG" "$S/flat" main --edit ch03.tex --trials "$TRIALS" --verify \
  --kinds "$KINDS" --out "$S/verify-whole-$ENG.jsonl" --quiet | tail -1
