#!/bin/sh
# corpus.sh PORT [OUT]: the makeindex corpus of this lane, oracle vs port.
#
# The .idx files come from TeX Live 2026's pdflatex (the oracle; never in the
# product path) on copies of the multi-pass index fixtures and the imakeidx
# book, plus TeX Live's own makeindex test inputs and styles at the source
# pin. Then docs/evidence/cold-speed-2026-10-04/makeindex/harness/corpus.py
# runs every .idx with no style, all of TeX Live's .ist files and the local
# styles, under four option sets, and compares everything byte for byte.
set -u
PORT=$1
O=${2:-${TMPDIR:-/tmp}/mkc}
W=$(cd "$(dirname "$0")/../../../.." && pwd)
H=$W/docs/evidence/cold-speed-2026-10-04/makeindex/harness
rm -rf "$O" && mkdir -p "$O/docs" "$O/tl"
ARGS=""
for d in makeindex-basic makeindex-see-and-pages bibtex-and-index-together; do
  mkdir -p "$O/docs/$d" && cp -R "$W/fixtures/multipass/$d/." "$O/docs/$d/"
  (cd "$O/docs/$d" && pdflatex -interaction=nonstopmode main.tex >/dev/null 2>&1)
  ARGS="$ARGS --idx $d=$O/docs/$d/main.idx"
done
mkdir -p "$O/docs/idxbook" && cp "$W/docs/evidence/p6-hyperopt-2026-10-04/raw/idx.tex" "$O/docs/idxbook/"
(cd "$O/docs/idxbook" && pdflatex -interaction=nonstopmode -shell-escape idx.tex >/dev/null 2>&1)
for f in "$O"/docs/idxbook/*.idx; do ARGS="$ARGS --idx idxbook-$(basename "$f" .idx)=$f"; done
PIN=6a300188053b8f2ded89dbd52293732a706b9c0e
BASE=https://raw.githubusercontent.com/TeX-Live/texlive-source/$PIN/texk/makeindexk/tests
for t in tort tortW sample nested-range nested-range-bb range pprecA pprecB romalpA romalpB romalpC romalpD toodeep; do
  curl -sfL -o "$O/tl/$t.idx" "$BASE/$t.idx" && ARGS="$ARGS --idx tl-test-$t=$O/tl/$t.idx"
done
for t in nested-range.ist pprec0.ist pprec1.ist pprec2.ist pprec3.ist pprec4.ist pprec5.ist pprec6.ist pprec7.ist pprec8.ist pprec9.ist pprecA.ist pprecB.ist romalpA.ist romalpB.ist; do
  curl -sfL -o "$O/tl/$t" "$BASE/$t" && ARGS="$ARGS --local-ist $O/tl/$t"
done
ARGS="$ARGS --local-ist $W/fixtures/multipass/makeindex-see-and-pages/dots.ist"
# shellcheck disable=SC2086
python3 "$H/corpus.py" --port "$PORT" $ARGS --jobs 4 --out "$O/corpus.jsonl"
