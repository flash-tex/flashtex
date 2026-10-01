#!/bin/bash
# zbuild.sh [PDF]: would another deflate give pdfTeX's bytes, and faster?
# Dumps PDF's streams (default: the full-300 benchmark PDF bench.py left) with
# zlib_identity.py (which also re-deflates them with Python's zlib), builds
# zb.c against TeX Live's zlib (third_party/zlib, what the engine links; -O3
# as the cc crate compiles it in release) and against the system's libz, and
# runs both, then zlib-rs and miniz_oxide (zrs/, from the cargo cache).
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
S=$W/docs/evidence/l6-optimizations-2026-09-29/scripts
Z=$W/third_party/zlib/zlib-src
B=/tmp/l6o/zbench
PDF=${1:-/tmp/l6o/time/u1-full-300/full-300.pdf}
rm -rf /tmp/l6o/zdump
python3 "$S/zlib_identity.py" "$PDF" --dump /tmp/l6o/zdump
mkdir -p $B/inc
cp "$Z/zconf.h.in" $B/inc/zconf.h
srcs=()
for f in adler32 compress crc32 deflate infback inffast inflate inftrees trees uncompr zutil; do srcs+=("$Z/$f.c"); done
cc -O3 -w -I$B/inc -I"$Z" -o $B/zb-texlive "$S/zb.c" "${srcs[@]}"
cc -O3 -w -o $B/zb-system "$S/zb.c" -lz
N=$(ls /tmp/l6o/zdump/*.raw | wc -l | tr -d ' ')
echo "texlive: $($B/zb-texlive /tmp/l6o/zdump $N)"
echo "system:  $($B/zb-system /tmp/l6o/zdump $N)"
mkdir -p /tmp/l6o/zrs/src
cp "$S/zrs/Cargo.toml" /tmp/l6o/zrs/
cp "$S/zrs/main.rs" /tmp/l6o/zrs/src/
(cd /tmp/l6o/zrs && cargo build -q --release --offline)
/tmp/l6o/zrs/target/release/zrs /tmp/l6o/zdump "$N"
