#!/usr/bin/env bash
# run.sh OUTDIR: lane MEMORY's measurement (2026-11-01), Linux (the NixOS runners). Builds the
# release host with `mem-stats` (DONE.mem carries the heap by tag), then, one host at a time,
# prof.py on plain-1000, full-1000, infdesc-x2, the metropolis deck and the 13 arXiv papers of
# P6-HYPEROPT; then four full-1000 hosts at once (hosts.py sums their Rss and Pss). With
# HEAPTRACK=1, also a build on glibc malloc with line tables, and heaptrack on full-1000 and
# infdesc-x2 (the peak's allocation sites). DOCS overrides the list of single-host runs.
set -euo pipefail
OUT=$1
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../../../.." && pwd)
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-$OUT/ib}
IB=$INCR_BENCH_DIR
cd "$W"
git rev-parse HEAD > "$OUT/HEAD"
ENG=${ENG:-prof}
if [ "${SKIP_BUILD:-0}" != 1 ]; then
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list --features flashtex-engine/mem-stats
  bash tools/incr-bench/mkeng.sh "$ENG"
fi
python3 tools/incr-bench/mkdocs.py >/dev/null
python3 tools/incr-bench/mkbook.py infdesc-x2 >/dev/null || echo "mkbook infdesc-x2 failed" >&2
J=$OUT/prof.jsonl
P=$S/prof.py
DOCS=${DOCS:-"plain-1000 full-1000 infdesc-x2 metropolis arxiv"}; DOCS=${DOCS//,/ }
for d in $DOCS; do
  case $d in
    plain-1000|full-1000)
      python3 "$P" "$ENG" "$d" "$J" --pages 0,300,600,999 --keys "${KEYS:-8}" || echo "$d failed" ;;
    infdesc-x2)
      ed=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("edit",""))' "$IB/docs/infdesc-x2/doc.json")
      python3 "$P" "$ENG" infdesc-x2 "$J" --src "$IB/docs/infdesc-x2" --edit "$ed" --pages 0,0 \
        --keys "${KEYS:-8}" || echo "infdesc-x2 failed" ;;
    lowmem)
      python3 "$P" "$ENG" full-1000 "$J" --pages 0,300,600,999 --keys "${KEYS:-8}" --profile low-memory || true
      ed=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("edit",""))' "$IB/docs/infdesc-x2/doc.json")
      python3 "$P" "$ENG" infdesc-x2 "$J" --src "$IB/docs/infdesc-x2" --edit "$ed" --pages 0,0 \
        --keys "${KEYS:-8}" --profile low-memory || true ;;
    metropolis)
      python3 "$P" "$ENG" metropolis "$J" --src "$W/fixtures/beamer-v3/metropolis" --pages 0,2 \
        --keys "${KEYS:-8}" || echo "metropolis failed" ;;
    arxiv)
      python3 "$S/arxiv13.py" > "$OUT/arxiv13.tsv" 2> "$OUT/arxiv13.err" || true
      while IFS=$'\t' read -r dir entry id; do
        [ -n "$dir" ] || continue
        python3 "$P" "$ENG" "arxiv-$id" "$J" --src "$dir" --main "$entry" --pages 0 --keys 4 \
          --timeout 900 || echo "arxiv-$id failed"
      done < "$OUT/arxiv13.tsv" ;;
  esac
done
if [ "${MULTI:-1}" = 1 ]; then
  # four projects open: each host its own copy of full-1000, all alive together at the end
  python3 "$S/hosts.py" "$OUT/hosts4.json" 3600 > "$OUT/hosts4.txt" &
  sp=$!
  pids=()
  for k in 1 2 3 4; do
    mkdir -p "$IB/docs/w$k"; cp "$IB/docs/full-1000/main.tex" "$IB/docs/w$k/"
    python3 "$P" "$ENG" "w$k" "$OUT/multi.jsonl" --src "$IB/docs/w$k" --pages 0,999 --keys 4 \
      --hold-s $((60 + 30 * (4 - k))) &
    pids+=($!)
    sleep 30
  done
  wait "${pids[@]}" || true
  touch "$OUT/hosts4.json.stop"; wait $sp || true
fi
if [ "${HEAPTRACK:-0}" = 1 ]; then
  HT=$(command -v heaptrack || true)
  if [ -z "$HT" ]; then echo "no heaptrack"; else
    CARGO_TARGET_DIR=${HT_TARGET:-$W/target-ht} CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
      cargo build --release --locked -p flashtex-engine -p flashtex-display-list --no-default-features \
      --features flashtex-engine/kpathsea,flashtex-engine/regex,flashtex-engine/distribution,flashtex-engine/makeindex,flashtex-engine/bibtex
    CARGO_TARGET_DIR=${HT_TARGET:-$W/target-ht} bash tools/incr-bench/mkeng.sh ht
    for d in ${HT_DOCS:-full-1000 infdesc-x2}; do
      extra=()
      if [ "$d" = infdesc-x2 ]; then
        ed=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("edit",""))' "$IB/docs/infdesc-x2/doc.json")
        extra=(--src "$IB/docs/infdesc-x2" --edit "$ed" --pages 0)
      else
        extra=(--pages 0,999)
      fi
      python3 "$P" ht "ht-$d" "$OUT/ht.jsonl" "${extra[@]}" --keys 4 --wrap "$HT -o $OUT/ht-$d" \
        $( [ "$d" != infdesc-x2 ] && echo "--src $IB/docs/$d" ) || echo "ht $d failed"
      f=$(ls "$OUT"/ht-"$d"*.zst "$OUT"/ht-"$d"*.gz 2>/dev/null | head -1 || true)
      [ -n "$f" ] && heaptrack_print -f "$f" --print-peaks 1 --print-allocators 0 --print-temporary 0 \
        --print-leaks 0 --peak-limit 40 --sub-peak-limit 6 > "$OUT/ht-$d.txt" 2>&1 || true
    done
  fi
fi
python3 - "$J" "$OUT" <<'EOF'
import json, sys
mb = lambda v: round((v or 0) / 2**20)
for line in open(sys.argv[1]):
    r = json.loads(line)
    m = (r.get('again') or r.get('typed') or {}).get('mem', {})
    top = sorted(((k, v) for k, v in m.items() if k.startswith('heap_') and not k.startswith('heap_peak')), key=lambda kv: -kv[1])[:6]
    print(f"{r['name']:24} open {mb(r['open_proc'].get('VmRSS')):5} typed {mb(r['typed_proc'].get('VmRSS')):5} "
          f"steady {mb(r['steady_proc'].get('VmRSS')):5} peak {mb(r['peak_hwm']):5} disk {mb(r['disk_s0']):4} | "
          + ' '.join(f"{k[5:]}={mb(v)}" for k, v in top))
EOF
