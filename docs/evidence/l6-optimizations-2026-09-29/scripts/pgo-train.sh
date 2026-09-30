#!/bin/bash
# pgo-train.sh ENGINE: run the instrumented engine /tmp/l6o/eng/ENGINE (built
# with -Cprofile-generate=/tmp/l6o/pgo-raw) over the training set -- every
# parity fixture (fixtures/real-world, fixtures/divergence-probes: 85 main.tex)
# and the 10- and 100-page benchmark documents, each compiled twice so the
# second run reads a settled .aux -- then merge the profile into
# /tmp/l6o/pgo.profdata. The 300/1,000-page documents are the test set and
# are never trained on.
set -e
# Every engine run is killed after ${LIMIT:-600} s (perl's alarm survives the
# exec), so an engine that loops cannot hang this script.
lim() { perl -e 'alarm shift @ARGV; exec @ARGV or die "exec: $!\n"' "${LIMIT:-600}" "$@"; }
W=$(cd "$(dirname "$0")/../../../.." && pwd)
E=$1
D=/tmp/l6o/eng/$E
rm -rf /tmp/l6o/pgo-raw /tmp/l6o/pgo-train; mkdir -p /tmp/l6o/pgo-train
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC FLASHTEX_POOL=$D/pdftex.pool FLASHTEX_FORMATS=$D/fmt
run() { # dir file
  (cd "$1" && for i in 1 2; do lim "$D/flashtex-initex" -fmt=pdflatex -interaction=batchmode "$2" >/dev/null 2>&1 || true; done)
}
n=0
for f in "$W"/fixtures/real-world/*/main.tex "$W"/fixtures/divergence-probes/*/main.tex; do
  t=/tmp/l6o/pgo-train/$(basename "$(dirname "$f")")
  mkdir -p "$t"; cp -R "$(dirname "$f")"/. "$t"/
  run "$t" main.tex; n=$((n+1))
done
for doc in plain-10 plain-100 full-10 full-100; do
  t=/tmp/l6o/pgo-train/$doc; mkdir -p "$t"; cp /tmp/l6o/docs/$doc.tex "$t"/
  run "$t" $doc.tex; n=$((n+1))
done
echo "trained on $n documents; $(ls /tmp/l6o/pgo-raw | wc -l) raw profiles"
PROFDATA=${PROFDATA:-$(xcrun -f llvm-profdata 2>/dev/null || command -v llvm-profdata)}
"$PROFDATA" merge -o /tmp/l6o/pgo.profdata /tmp/l6o/pgo-raw
ls -la /tmp/l6o/pgo.profdata
