#!/usr/bin/env bash
# restart_profile.sh WORK [KEYS]: where a keystroke's time goes in the Unicode host against the
# pdfTeX (Classic) host on the same edit (lane UNICODE), on the NixOS PC's self-hosted runner
# (the runner service is in flashtex.slice; everything here runs under nice).
#
#   1. mode_gap.py (tools/unicode-m3): T7's full-120 typed into by dl3-keys through both hosts,
#      20 keystrokes in the middle and near the end: wall times to the edited page.
#   2. For classic and unicode-fontspec at both places: a host is started, the document opened
#      and typed into once by dl3-keys, then `perf record` (cycles:u) attaches to the host while
#      dl3-keys types KEYS more letters (their compile is all the profile holds: the second open
#      on the connection finds nothing changed). Top symbols, flat, by DSO and symbol.
#
# Writes WORK/r/gap.txt, WORK/r/perf-<variant>-<at>.txt, WORK/r/keys-<variant>-<at>.jsonl.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
KEYS=${2:-60}
mkdir -p "$WORK/r"
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$TMPDIR" "$INCR_BENCH_DIR"
T=${CARGO_TARGET_DIR:-$ROOT/target}
if command -v perf >/dev/null 2>&1; then PERF_CMD=(perf); else PERF_CMD=(nix shell nixpkgs#perf -c perf); fi

(cd "$ROOT" && nice -n 5 cargo build --release --locked -p flashtex-engine -p flashtex-display-list)
(cd "$ROOT" && bash tools/incr-bench/mkeng.sh cur >/dev/null)
(cd "$ROOT/crates/flashtex-xetex" && CARGO_TARGET_DIR="$T" nice -n 5 cargo build --release --locked --bin flashtex-host-unicode)
UH="$T/release/flashtex-host-unicode"
TL="$(kpsewhich -var-value TEXMFDIST)"
export FLASHTEX_FONT_DIRS="$TL/fonts/opentype:$TL/fonts/truetype"
export FLASHTEX_CACHE_DIR="$WORK/flashtex-cache"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
echo "Unicode host $(git -C "$ROOT" rev-parse --short HEAD); $(nproc) cpus; load1 $(cut -d' ' -f1 /proc/loadavg)"

[ "${GAP:-1}" = 0 ] || nice -n 5 python3 "$ROOT/tools/unicode-m3/mode_gap.py" --classic "$INCR_BENCH_DIR/cur" --unicode-host "$UH" \
  --keys 20 --at 0.5,0.98 --variants classic,classic-nohook,unicode,unicode-nohook,unicode-fontspec \
  --out "$WORK/gap" --json "$WORK/r/gap.json" 2>&1 | tee "$WORK/r/gap.txt"
# Classic without its replays (intrinsics.rs, boxmemo.rs): what they save it a page
[ "${GAP:-1}" = 0 ] || FLASHTEX_INTRINSICS=off FLASHTEX_BOXMEMO=off nice -n 5 python3 "$ROOT/tools/unicode-m3/mode_gap.py" \
  --classic "$INCR_BENCH_DIR/cur" --unicode-host "$UH" --keys 20 --at 0.5,0.98 --variants classic,classic-nohook \
  --out "$WORK/gap-off" --json "$WORK/r/gap-off.json" 2>&1 | tee "$WORK/r/gap-off.txt"

prof() {
  local variant=$1 at=$2 d="$WORK/p-$1-$2" sock="$TMPDIR/p-$$.sock" exe envs=()
  rm -rf "$d"; mkdir -p "$d/out"
  python3 -c "import sys; sys.path.insert(0, '$ROOT/tools/unicode-m3'); import mode_gap as m; \
open('$d/main.tex', 'w').write(m.source('$variant', 120))"
  if [[ $variant == classic* ]]; then
    exe="$INCR_BENCH_DIR/cur/flashtex-host"
    envs=(FLASHTEX_POOL="$INCR_BENCH_DIR/cur/pdftex.pool" FLASHTEX_FORMATS="$INCR_BENCH_DIR/fmt-cur")
  else
    exe="$UH"
    envs=(FLASHTEX_UNICODE_STAGES=1 FLASHTEX_FSWATCH_DEBUG=1)
  fi
  rm -f "$sock"
  env "${envs[@]}" nice -n 5 "$exe" --socket "$sock" --s0-cache "$d/s0" > "$d/host.out" 2> "$d/host.err" &
  local hp=$!
  for _ in $(seq 1 600); do grep -q listening "$d/host.out" 2>/dev/null && break; sleep 0.1; done
  # one session: the open, then KEYS keystrokes; perf attaches once the
  # first two keystrokes are done (the open and the checkpoints near the
  # edit are behind it), so the profile holds keystrokes only
  local out="$WORK/r/keys-$variant-$at.jsonl"
  nice -n 5 "$INCR_BENCH_DIR/cur/dl3-keys" --socket "$sock" --root "$d" --main main.tex --output-dir "$d/out" \
    --keys "$KEYS" --gap-ms 20 --at "$at" > "$out" 2> "$d/keys.err" &
  local kp=$!
  for _ in $(seq 1 3000); do [ "$(grep -c '"key"' "$out" 2>/dev/null)" -ge 2 ] && break; sleep 0.05; done
  "${PERF_CMD[@]}" record --clockid monotonic -e ${PERF_EVENT:-cycles:u} -F ${PERF_HZ:-9999} ${PERF_G:-} -p "$hp" -o "$d/perf.data" > "$d/perf.log" 2>&1 &
  local pp=$!
  wait "$kp" || true
  kill -INT "$pp" 2>/dev/null || true
  wait "$pp" 2>/dev/null || true
  kill "$hp" 2>/dev/null || true
  wait "$hp" 2>/dev/null || true
  {
    echo "== $variant at $at: $(grep summary "$WORK/r/keys-$variant-$at.jsonl" | cut -c1-400)"
    if [[ $variant != classic* ]]; then python3 "$ROOT/tools/unicode-m3/stages_summary.py" "$d/host.err"; python3 "$ROOT/tools/unicode-m3/abs_summary.py" "$d/host.err"; \
      "${PERF_CMD[@]}" script -i "$d/perf.data" -F comm,time,ip,sym --ns 2>/dev/null > "$d/script.txt"; \
      python3 "$ROOT/tools/unicode-m3/win_profile.py" "$d/host.err" "$d/script.txt" pre; \
      python3 "$ROOT/tools/unicode-m3/win_profile.py" "$d/host.err" "$d/script.txt" page; fi
    "${PERF_CMD[@]}" report -i "$d/perf.data" --no-children --sort dso,symbol --stdio 2>/dev/null \
      | grep -v '^#' | grep -v '^$' | sort -t% -k1 -rn | head -90
  } > "$WORK/r/perf-$variant-$at.txt" || true
  grep -v '^\[stage\] page' "$d/host.err" | tail -200 > "$WORK/r/host-err-$variant-$at.txt" || true
  { echo "fswatch lines by kind:"; grep '^\[fswatch\]' "$d/host.err" | sed -E 's/^\[fswatch\] (stat|key dir)[^:]*(: ([a-z ]+))?.*/\1 \3/' | sort | uniq -c | sort -rn | head; \
    echo "most stat'ed:"; grep '^\[fswatch\] stat ' "$d/host.err" | sed -E 's/: [a-z ]+$//' | sort | uniq -c | sort -rn | head -25; } \
    > "$WORK/r/fswatch-$variant-$at.txt" 2>/dev/null || true
  cat "$WORK/r/perf-$variant-$at.txt" | head -45
  [ -n "${KEEP:-}" ] || rm -rf "$d"
}

for at in ${ATS:-0.5 0.98}; do
  for v in ${VARIANTS:-classic unicode-fontspec}; do
    prof "$v" "$at"
  done
done
