#!/usr/bin/env bash
# kw.sh OUT: where a key that comes during a VIEWPORT's idle re-densify waits (lane MEMORY,
# measurement only). Builds this checkout as engine "prof" (unless NOBUILD=1), then ROUNDS rounds
# of full-1000 jumps 0 -> 300 -> 600 at FLASHTEX_FAR=1 for each delay in DELAYS (the key that many
# ms after the VIEWPORT; the re-densify starts after its 50 ms grace), with FLASHTEX_KEYWAIT_DEBUG:
# the host's and the client's `[kw]` lines go to OUT/kw.log, the keys to OUT/jumps.jsonl.
set -euo pipefail
OUT=$1
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../../../.." && pwd)
export INCR_BENCH_DIR=${INCR_BENCH_DIR:?}
IB=$INCR_BENCH_DIR
cd "$W"
git rev-parse HEAD > "$OUT/HEAD"
if [ "${NOBUILD:-}" != 1 ]; then
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list
  bash tools/incr-bench/mkeng.sh "${ENG:-prof}"
fi
[ -f "$IB/docs/full-1000/main.tex" ] || python3 tools/incr-bench/mkdocs.py >/dev/null
for r in $(seq 1 "${ROUNDS:-2}"); do
  for d in ${DELAYS:-80 65 100}; do
    FLASHTEX_FAR=1 FLASHTEX_KEYWAIT_DEBUG=1 JUMPS_LOG=$OUT/kw.log \
      python3 "$S/jumps.py" "${ENG:-prof}" full-1000 "$OUT/jumps.jsonl" --cfg "vp$d" --round "$r" \
      --jump-ms "$d" --keys "${KEYS:-4}" --pages "${PAGES:-0,300,600}" || echo "vp$d r$r failed"
  done
done
