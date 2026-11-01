#!/usr/bin/env bash
# Socket round-trip measurements for display-list-v3 (lane P3-DISPLAYLIST).
#
#   tools/displaylist/bench_socket.sh FMTDIR [OUT.jsonl]
#
# 1. Real compiles: flashtex-host runs the engine on parity fixtures; the
#    reference client (dl3-client) measures, from sending COMPILE, the time
#    to STARTED, to the first PAGE and to DONE, five compiles each, the
#    first cold (every font program sent) and the rest with --reuse-fonts.
# 2. Transport alone: the host runs a stand-in "engine" that replays the
#    display lists of every parity fixture (target/dl3-positions, written by
#    check_positions.py) onto descriptor 3 as fast as it can; the client's
#    bytes / (DONE - STARTED) is what the host relay, the socket and the
#    client's decoding sustain together.
#
# Needs a release build (cargo build --release -p flashtex-engine
# -p flashtex-display-list) and FMTDIR holding the engine's pdflatex.fmt.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fmt=${1:?usage: bench_socket.sh FMTDIR [OUT.jsonl]}
out=${2:-/dev/stdout}
bin=${FLASHTEX_BIN_DIR:-$root/target/release}
export FLASHTEX_POOL=$root/crates/flashtex-engine/pdftex.pool FLASHTEX_FORMATS=$fmt
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
work=$(mktemp -d)
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$work"' EXIT

sock=$work/host.sock
"$bin/flashtex-host" --socket "$sock" >"$work/host.log" 2>&1 &
for _ in $(seq 50); do [ -S "$sock" ] && break; sleep 0.1; done
: >"$out.tmp"
for f in plain-article hyperref-toc lecture-notes thesis-chapter beamer-madrid; do
    mkdir -p "$work/$f"
    cp -R "$root/fixtures/real-world/$f/." "$work/$f/"
    # Converge the .aux files first, as an editing session would have.
    "$bin/dl3-client" --socket "$sock" --root "$work/$f" --main main.tex --repeat 2 --quiet >/dev/null
    "$bin/dl3-client" --socket "$sock" --root "$work/$f" --main main.tex --repeat 5 --reuse-fonts --quiet |
        sed "s/^{/{\"fixture\":\"$f\",/" >>"$out.tmp"
done
kill %1 2>/dev/null || true
wait 2>/dev/null || true

# Transport: replay every fixture's display list through the host.
replay=$work/all.dl3
cat "$root"/target/dl3-positions/*/display.dl3 >"$replay"
cat >"$work/replay-engine" <<EOF
#!/bin/sh
exec cat "$replay" >&3
EOF
chmod +x "$work/replay-engine"
sock2=$work/replay.sock
"$bin/flashtex-host" --socket "$sock2" --engine "$work/replay-engine" >"$work/host2.log" 2>&1 &
for _ in $(seq 50); do [ -S "$sock2" ] && break; sleep 0.1; done
mkdir -p "$work/empty" && touch "$work/empty/main.tex"
"$bin/dl3-client" --socket "$sock2" --root "$work/empty" --main main.tex --repeat 5 --quiet |
    sed 's/^{/{"fixture":"replay-all-82",/' >>"$out.tmp"
python3 - "$out.tmp" <<'PY'
import json, sys, statistics
rows = [json.loads(l) for l in open(sys.argv[1])]
by = {}
for r in rows:
    by.setdefault(r["fixture"], []).append(r)
for f, rs in by.items():
    warm = rs[1:] if len(rs) > 1 else rs
    med = lambda k: statistics.median(r[k] for r in warm if r.get(k) is not None)
    line = {"fixture": f, "pages": rs[0]["pages"], "first_compile_bytes": rs[0]["bytes"],
            "warm_bytes": warm[0]["bytes"], "median_first_page_ms": round(med("first_page_ms"), 1),
            "median_done_ms": round(med("done_ms"), 1), "median_decode_mb_per_s": round(med("decode_mb_per_s"))}
    if f.startswith("replay"):
        line["transport_mb_per_s"] = round(statistics.median(
            r["bytes"] / 1e6 / ((r["done_ms"] - r["started_ms"]) / 1000) for r in warm))
    print(json.dumps(line))
PY
cat "$out.tmp" >"$out.raw" 2>/dev/null || true
rm -f "$out.tmp"
