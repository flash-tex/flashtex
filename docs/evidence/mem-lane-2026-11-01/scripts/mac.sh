#!/usr/bin/env bash
# mac.sh OUTDIR: lane MEMORY's VIEWPORT gate in milliseconds, on one machine (the hosted M1 of
# mem-lane-mac.yml), interleaved. Builds this checkout as engine "prof" and BASE_REF (main) as
# "base", then ROUNDS rounds; in each, every document and every configuration once, the order
# rotated by the round:
#   main     base, as main retains (every page checkpoint)
#   far2     prof, FLASHTEX_FAR=2, no VIEWPORT
#   vp300    prof, FLASHTEX_FAR=4, VIEWPORT then 300 ms before each jump's first key
#   vp0      prof, FLASHTEX_FAR=4, VIEWPORT and the first key at once
# full-1000 jumps 0 -> 300 -> 600; infdesc-x2 types in a chapter, its copy ~570 pages on, and back.
set -euo pipefail
OUT=$1
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../../../.." && pwd)
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-/tmp/ib}
IB=$INCR_BENCH_DIR
cd "$W"
git rev-parse HEAD > "$OUT/HEAD"
cargo build --release --locked -p flashtex-engine -p flashtex-display-list
bash tools/incr-bench/mkeng.sh prof
rm -rf "$OUT/base"; git fetch -q --depth=1 origin "$BASE_REF" && git worktree add --detach "$OUT/base" FETCH_HEAD
(cd "$OUT/base" && git rev-parse HEAD > "$OUT/BASE" && CARGO_TARGET_DIR="$W/target-base" cargo build --release --locked \
   -p flashtex-engine -p flashtex-display-list && CARGO_TARGET_DIR="$W/target-base" bash tools/incr-bench/mkeng.sh base)
export JUMPS_CLIENT=$IB/prof/dl3-keys
python3 tools/incr-bench/mkdocs.py >/dev/null
python3 tools/incr-bench/mkbook.py infdesc-x2 >/dev/null
ed=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("edit",""))' "$IB/docs/infdesc-x2/doc.json")
J=$OUT/jumps.jsonl
CFGS=(main far2 vp300 vp0)
run() {  # run CFG ROUND DOC
  local c=$1 r=$2 d=$3 eng=prof far= jm=-1 extra=()
  case $c in
    main) eng=base ;;
    far2) far=2 ;;
    vp300) far=4; jm=300 ;;
    vp0) far=4; jm=0 ;;
  esac
  case $d in
    full-1000) extra=(--pages 0,300,600) ;;
    infdesc-x2) extra=(--src "$IB/docs/infdesc-x2" --pages 0,0,0 --edit "$ed,x2/$ed,$ed") ;;
  esac
  FLASHTEX_FAR=$far python3 "$S/jumps.py" "$eng" "$d" "$J" --cfg "$c" --round "$r" --jump-ms "$jm" \
    --keys "${KEYS:-6}" "${extra[@]}" || echo "$d $c r$r failed"
}
for r in $(seq 1 "${ROUNDS:-3}"); do
  for d in ${DOCS:-full-1000 infdesc-x2}; do
    for k in 0 1 2 3; do
      run "${CFGS[$(( (k + r) % 4 ))]}" "$r" "$d"
    done
  done
done
python3 "$S/jumps_sum.py" "$J" | tee "$OUT/summary.txt"
